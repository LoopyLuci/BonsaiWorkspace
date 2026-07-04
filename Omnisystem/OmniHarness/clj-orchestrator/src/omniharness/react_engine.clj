(ns omniharness.react-engine
  "ReAct (Reason+Act) loop with native tool-call support and policy checks."
  (:require [clojure.core.async    :as async :refer [go <! >! chan timeout]]
            [clojure.string        :as str]
            [cheshire.core         :as json]
            [taoensso.timbre       :as log]
            [omniharness.events    :as events]
            [omniharness.policy    :as policy]
            [omniharness.client    :refer [model-stub tool-stub]])
  (:import [omniharness.v1
            ChatRequest ChatMessage ToolDef
            ToolExecuteRequest]))

;; ── LLM call via gRPC ModelService ──────────────────────────────────────────

(defn llm-chat
  "Send a chat request to the kernel model service. Returns response map."
  [{:keys [model-id messages temperature max-tokens system tools]
    :or   {temperature 0.7 max-tokens 4096 model-id "claude-sonnet-4-6"}}]
  (let [proto-msgs (map (fn [{:keys [role content]}]
                          (-> (ChatMessage/newBuilder)
                              (.setRole    (str role))
                              (.setContent (str content))
                              .build))
                        messages)
        req-builder (-> (ChatRequest/newBuilder)
                        (.setModelId    model-id)
                        (.setTemperature (float temperature))
                        (.setMaxTokens  (int max-tokens)))
        _  (when system (.setSystem req-builder system))
        _  (doseq [msg proto-msgs] (.addMessages req-builder msg))
        _  (doseq [{:keys [name description schema]} tools]
             (.addTools req-builder
                        (-> (ToolDef/newBuilder)
                            (.setName name) (.setDescription description)
                            (.setInputSchema (or schema "{}"))
                            .build)))
        resp (.chat @model-stub (.build req-builder))]
    {:content       (.getContent resp)
     :model-used    (.getModelUsed resp)
     :finish-reason (.getFinishReason resp)
     :input-tokens  (.getInputTokens resp)
     :output-tokens (.getOutputTokens resp)
     :tool-calls    (mapv (fn [tc]
                            {:id        (.getId tc)
                             :name      (.getName tc)
                             :arguments (try (json/parse-string (.getArguments tc) true)
                                            (catch Exception _ {}))})
                          (.getToolCallsList resp))
     :latency-ms    (.getLatencyMs resp)}))

;; ── Tool execution via gRPC ToolService ─────────────────────────────────────

(defn execute-tool!
  "Execute a tool via gRPC kernel. Returns result string."
  [tool-name args]
  (let [req (-> (ToolExecuteRequest/newBuilder)
                (.setName      (str tool-name))
                (.setArguments (if (string? args) args (json/generate-string args)))
                (.setTimeoutMs 30000)
                .build)
        ^omniharness.v1.ToolExecuteResponse resp (.execute @tool-stub req)]
    (if (.getSuccess resp)
      (.getResult resp)
      (str "Tool error: " (.getError resp)))))

;; ── Text-format ReAct parser ─────────────────────────────────────────────────

(defn parse-react-text [text]
  (let [thought-m (re-find #"(?i)Thought:\s*(.*?)(?=Action:|$)" text)
        action-m  (re-find #"(?i)Action:\s*(\w[\w\s]*?)(?=ActionInput:|$|\n)" text)
        input-m   (re-find #"(?i)ActionInput:\s*([\s\S]*?)$" text)]
    {:thought      (if thought-m (str/trim (second thought-m)) text)
     :action       (if action-m  (str/trim (second action-m))  "FinalAnswer")
     :action-input (when input-m
                     (try (json/parse-string (str/trim (second input-m)) true)
                          (catch Exception _ {:answer (str/trim (second input-m))})))}))

;; ── Main ReAct loop ──────────────────────────────────────────────────────────

(def system-prompt
  "You are a capable AI assistant with tools. Use Thought/Action/ActionInput format.
When done, use Action: FinalAnswer with ActionInput: {\"answer\": \"...\"}.")

(defrecord ReActStep [step thought action action-input observation latency-ms used-native?])
(defrecord ReActResult [answer steps success total-tokens elapsed-ms])

(defn run-react-loop
  "Run a ReAct loop. Returns a channel that delivers ReActResult."
  [{:keys [objective model-id max-steps temperature tools session-id]
    :or   {model-id "claude-sonnet-4-6" max-steps 20 temperature 0.7
           tools [] session-id ""}}]
  (go
    (let [start-ms  (System/currentTimeMillis)
          history   (atom [{:role "user" :content (str "Task: " objective)}])
          steps     (atom [])
          tokens    (atom 0)]
      (loop [step 0]
        (if (>= step max-steps)
          ;; Force final answer
          (let [resp (llm-chat {:model-id model-id :messages @history :temperature 0.3
                                :max-tokens 512 :system system-prompt})]
            (->ReActResult (:content resp) @steps false @tokens
                           (- (System/currentTimeMillis) start-ms)))

          (let [t0       (System/currentTimeMillis)
                resp     (llm-chat {:model-id model-id :messages @history
                                    :temperature temperature :system system-prompt
                                    :tools tools})
                _        (swap! tokens + (:input-tokens resp) (:output-tokens resp))
                lat      (- (System/currentTimeMillis) t0)]

            ;; Native tool_calls
            (if-let [tc (first (:tool-calls resp))]
              (let [{:keys [name arguments]} tc
                    _  (swap! history conj {:role "assistant" :content (or (:content resp) "")})
                    ok (policy/allowed? name arguments)
                    obs (if ok
                          (execute-tool! name arguments)
                          (str "Policy denied: " name))
                    _  (swap! history conj {:role "tool" :content obs})
                    _  (events/append-event! "react" "ToolExecuted"
                                             {:tool name :ok ok} session-id)
                    _  (swap! steps conj
                              (->ReActStep step (:content resp) name arguments obs lat true))]
                (if (= (str/lower-case name) "finalanswer")
                  (->ReActResult (get arguments :answer (str arguments))
                                 @steps true @tokens (- (System/currentTimeMillis) start-ms))
                  (recur (inc step))))

              ;; Text-format ReAct
              (let [{:keys [thought action action-input]} (parse-react-text (:content resp))
                    _  (swap! history conj {:role "assistant" :content (:content resp)})]
                (if (contains? #{"finalanswer" "final_answer" "final answer"}
                               (str/lower-case action))
                  (let [answer (or (get action-input :answer) (str action-input))]
                    (->ReActResult answer @steps true @tokens
                                   (- (System/currentTimeMillis) start-ms)))
                  (let [ok  (policy/allowed? action action-input)
                        obs (if ok
                              (execute-tool! action action-input)
                              (str "Policy denied: " action))
                        _   (swap! history conj {:role "user" :content (str "Observation: " obs)})
                        _   (events/append-event! "react" "Step"
                                                  {:step step :action action} session-id)
                        _   (swap! steps conj
                                   (->ReActStep step thought action action-input obs lat false))]
                    (recur (inc step))))))))))))
