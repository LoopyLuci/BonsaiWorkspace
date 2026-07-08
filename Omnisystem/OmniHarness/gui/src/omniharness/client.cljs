(ns omniharness.client
  "HTTP + WebSocket client — wraps Python REST API and Rust gRPC via gateway"
  (:require [cljs-http.client :as http]
            [cljs.core.async :refer [go <!]]
            [re-frame.core :as rf]))

(def ^:private base-url (atom "http://localhost:8080"))

(defn set-base-url! [url] (reset! base-url url))

;; ── HTTP helpers ──────────────────────────────────────────────────────────────

(defn- api [path] (str @base-url path))

(defn- ->result [response]
  (if (< (:status response) 400)
    {:ok true  :data (:body response)}
    {:ok false :error (get-in response [:body :detail] "Request failed")}))

;; ── Chat ──────────────────────────────────────────────────────────────────────

(defn chat! [{:keys [model-id messages temperature max-tokens session-id]}]
  (go (->result
       (<! (http/post (api "/api/chat")
                      {:json-params {:model_id    model-id
                                     :messages    messages
                                     :temperature (or temperature 0.7)
                                     :max_tokens  (or max-tokens 4096)
                                     :session_id  session-id}
                       :with-credentials? false})))))

(defn agent-run! [{:keys [objective model-id max-steps session-id]}]
  (go (->result
       (<! (http/post (api "/api/agent/run")
                      {:json-params {:objective  objective
                                     :model_id   model-id
                                     :max_steps  (or max-steps 20)
                                     :session_id session-id}
                       :with-credentials? false})))))

;; ── Sessions ──────────────────────────────────────────────────────────────────

(defn create-session! [model-id]
  (go (->result
       (<! (http/post (api "/api/sessions")
                      {:json-params {:model_id model-id}
                       :with-credentials? false})))))

(defn get-session! [session-id]
  (go (->result (<! (http/get (api (str "/api/sessions/" session-id))
                              {:with-credentials? false})))))

(defn list-sessions! []
  (go (->result (<! (http/get (api "/api/sessions") {:with-credentials? false})))))

(defn delete-session! [session-id]
  (go (->result (<! (http/delete (api (str "/api/sessions/" session-id))
                                 {:with-credentials? false})))))

;; ── Memory ────────────────────────────────────────────────────────────────────

(defn memory-store! [{:keys [collection content metadata]}]
  (go (->result
       (<! (http/post (api "/api/memory/store")
                      {:json-params {:collection collection
                                     :content    content
                                     :metadata   (or metadata {})}
                       :with-credentials? false})))))

(defn memory-search! [{:keys [collection query top-k]}]
  (go (->result
       (<! (http/post (api "/api/memory/search")
                      {:json-params {:collection collection
                                     :query      query
                                     :top_k      (or top-k 5)}
                       :with-credentials? false})))))

;; ── Tools ─────────────────────────────────────────────────────────────────────

(defn list-tools! []
  (go (->result (<! (http/get (api "/api/tools") {:with-credentials? false})))))

(defn execute-tool! [{:keys [name arguments]}]
  (go (->result
       (<! (http/post (api "/api/tools/execute")
                      {:json-params {:name name :arguments arguments}
                       :with-credentials? false})))))

;; ── Models ────────────────────────────────────────────────────────────────────

(defn list-models! []
  (go (->result (<! (http/get (api "/api/models") {:with-credentials? false})))))

(defn health! []
  (go (->result (<! (http/get (api "/api/health") {:with-credentials? false})))))

;; ── WebSocket streaming ───────────────────────────────────────────────────────

(defn connect-ws!
  "Opens a WebSocket for streaming chat. Calls on-chunk with each delta string,
   on-done when the stream completes, on-error on failure."
  [{:keys [session-id on-chunk on-done on-error]}]
  (let [ws-url (-> @base-url
                   (clojure.string/replace "http://" "ws://")
                   (clojure.string/replace "https://" "wss://")
                   (str "/ws/chat/" session-id))
        ws     (js/WebSocket. ws-url)]
    (set! (.-onmessage ws)
          (fn [e]
            (let [data (js->clj (js/JSON.parse (.-data e)) :keywordize-keys true)]
              (case (:type data)
                "chunk" (on-chunk (:delta data))
                "done"  (on-done (:content data))
                "error" (on-error (:message data))
                nil))))
    (set! (.-onerror ws)
          (fn [_] (on-error "WebSocket connection failed")))
    ws))
