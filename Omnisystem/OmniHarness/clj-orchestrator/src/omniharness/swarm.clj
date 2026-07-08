(ns omniharness.swarm
  "Agent swarm coordinator — pipeline, parallel/map-reduce, orchestrator-workers,
   and debate topologies over a shared blackboard. `llm-fn` is
   (fn [model messages system] -> string). Every step passes through the governor."
  (:require [clojure.string :as str]
            [cheshire.core :as json]
            [omniharness.governance :as gov]))

;; ── Shared blackboard ────────────────────────────────────────────────────────

(defn new-board [] (atom {:data {} :log []}))
(defn board-post! [board agent k v]
  (swap! board #(-> % (assoc-in [:data k] v) (update :log conj {:agent agent :key k}))))
(defn board-read [board k] (get-in @board [:data k]))

;; ── Single agent run ─────────────────────────────────────────────────────────

(defn- tokens-est [^String s] (max 1 (quot (count s) 4)))

(defn- run-agent [llm-fn governor board {:keys [id system model] :as _agent} prompt]
  (gov/checkpoint! governor (str "agent:" id))
  (gov/check-model! governor model)
  (let [text (llm-fn model [{:role "user" :content prompt}] system)]
    (gov/record-call! governor model (+ (tokens-est prompt) (tokens-est text)))
    (board-post! board id (str id ":last") text)
    text))

;; ── Topologies ───────────────────────────────────────────────────────────────

(defn- pipeline [llm-fn governor board agents task]
  (reduce (fn [current agent]
            (run-agent llm-fn governor board agent
                       (if (= current task) task
                           (str "Task: " task "\n\nPrevious stage:\n" current
                                "\n\nImprove it."))))
          task agents))

(defn- parallel [llm-fn governor board agents task reducer]
  (let [results (mapv (fn [a] [a (future (run-agent llm-fn governor board a task))]) agents)
        outs    (mapv (fn [[a f]] {:agent (:id a) :output @f}) results)
        combined (str/join "\n\n" (map #(str "### " (:agent %) "\n" (:output %)) outs))]
    (if reducer
      (run-agent llm-fn governor board reducer
                 (str "Task: " task "\n\nResults:\n" combined "\n\nMerge into one."))
      combined)))

(defn- parse-json-list [text]
  (try
    (when-let [m (re-find #"(?s)\[.*\]" text)]
      (let [v (json/parse-string m)]
        (when (vector? v) (mapv str v))))
    (catch Exception _ nil)))

(defn- orchestrator-workers [llm-fn governor board agents task]
  (let [lead    (or (first (filter #(= "orchestrator" (:role %)) agents)) (first agents))
        workers (or (seq (remove #(= (:id %) (:id lead)) agents)) agents)
        plan    (run-agent llm-fn governor board lead
                           (str "Split into 2-5 parallel subtasks as a JSON array.\n\nTask: " task))
        subs    (or (parse-json-list plan) [task])
        results (mapv (fn [i sub]
                        (let [w (nth workers (mod i (count workers)))]
                          {:subtask sub
                           :output (future (run-agent llm-fn governor board w
                                                      (str "Subtask: " sub "\n\nGoal: " task)))}))
                      (range) subs)
        findings (str/join "\n\n" (map #(str "[" (:subtask %) "] -> " @(:output %)) results))]
    (run-agent llm-fn governor board lead
               (str "Task: " task "\n\nWorker results:\n" findings "\n\nSynthesize the deliverable."))))

(defn- debate [llm-fn governor board agents task rounds]
  (let [open (into {} (map (fn [a] [(:id a) (run-agent llm-fn governor board a
                                                       (str "Question: " task "\n\nAnswer with reasoning."))])
                           agents))
        final (reduce
               (fn [positions _round]
                 (into {} (map (fn [a]
                                 (let [others (str/join "\n\n" (for [[k v] positions :when (not= k (:id a))]
                                                                 (str k ": " v)))]
                                   [(:id a) (run-agent llm-fn governor board a
                                                       (str "Question: " task "\n\nOthers:\n" others
                                                            "\n\nCritique and refine your answer."))]))
                               agents)))
               open (range rounds))
        chair (first agents)]
    (run-agent llm-fn governor board chair
               (str "Question: " task "\n\nFinal positions:\n"
                    (str/join "\n\n" (for [[k v] final] (str k ": " v)))
                    "\n\nState the consensus."))))

;; ── Public entry point ───────────────────────────────────────────────────────

(defn run-swarm
  "topology ∈ #{:pipeline :parallel :orchestrator :debate}.
   opts: {:governor g :reducer agent :rounds n}. Returns {:output s :usage u :blackboard m}."
  [llm-fn topology agents task {:keys [governor reducer rounds] :or {rounds 2}}]
  (let [governor (or governor (gov/new-governor))
        board    (new-board)]
    (gov/audit-append! (:audit @governor) "swarm_start" {:topology topology :agents (mapv :id agents)})
    (let [output (case topology
                   :pipeline     (pipeline llm-fn governor board agents task)
                   :parallel     (parallel llm-fn governor board agents task reducer)
                   :orchestrator (orchestrator-workers llm-fn governor board agents task)
                   :debate       (debate llm-fn governor board agents task rounds)
                   (throw (ex-info (str "unknown topology " topology) {})))]
      (gov/audit-append! (:audit @governor) "swarm_end" {:usage (:usage @governor)})
      {:output output :usage (:usage @governor) :blackboard (:data @board)})))
