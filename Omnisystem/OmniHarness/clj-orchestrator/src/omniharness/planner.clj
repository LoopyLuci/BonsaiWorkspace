(ns omniharness.planner
  "Hierarchical Task Network (HTN) planner with forward-chaining search."
  (:require [clojure.string :as str]
            [taoensso.timbre :as log]))

;; ── Data types ───────────────────────────────────────────────────────────────

(defrecord Task    [name params is-primitive? priority])
(defrecord Operator [name preconditions effects execute-fn])
(defrecord Method  [compound-name condition decompose])

(defn make-task
  ([name] (make-task name {} true 0))
  ([name params] (make-task name params true 0))
  ([name params primitive?] (make-task name params primitive? 0))
  ([name params primitive? priority]
   (->Task name params primitive? priority)))

;; ── Planner state ─────────────────────────────────────────────────────────────

(def ^:private operators (atom {}))
(def ^:private methods   (atom {}))

(defn register-operator! [{:keys [name] :as op}]
  (swap! operators assoc name op)
  name)

(defn register-method! [{:keys [compound-name] :as method}]
  (swap! methods update compound-name (fnil conj []) method)
  compound-name)

;; ── Planning algorithm ────────────────────────────────────────────────────────

(defn- seek-plan [tasks state plan]
  (if (empty? tasks)
    plan
    (let [[head & tail] tasks]
      (if (or (:is-primitive? head) (contains? @operators (:name head)))
        ;; Primitive task
        (let [op (get @operators (:name head))]
          (if (or (nil? op) ((:preconditions op) state))
            (let [new-state (if op ((:effects op) state (:params head)) state)]
              (seek-plan tail (merge state new-state) (conj plan head)))
            nil))
        ;; Compound task — try each applicable method
        (some (fn [method]
                (when ((:condition method) state (:params head))
                  (let [subtasks ((:decompose method) state (:params head))]
                    (seek-plan (concat subtasks tail) state plan))))
              (get @methods (:name head) []))))))

(defn plan
  "Plan for a task given a world state. Returns sequence of primitive tasks or nil."
  [task state]
  (seek-plan [task] state []))

(defn plan-goal
  "Plan for a goal string (wraps in a Task). Returns primitive plan or nil."
  [goal-name params state]
  (plan (make-task goal-name params false) state))

;; ── Execution ─────────────────────────────────────────────────────────────────

(defn execute-plan!
  "Execute a plan (list of primitive tasks) sequentially. Returns {:results [...] :state ...}."
  [tasks state]
  (loop [remaining tasks
         cur-state  state
         results    []]
    (if (empty? remaining)
      {:results results :state cur-state}
      (let [task (first remaining)
            op   (get @operators (:name task))]
        (if op
          (let [result    (try
                            ((:execute-fn op) cur-state (:params task))
                            (catch Exception e (str "Error: " (.getMessage e))))
                new-state (merge cur-state ((:effects op) cur-state (:params task)))]
            (recur (rest remaining) new-state
                   (conj results {:task (:name task) :result result :ok true})))
          (recur (rest remaining) cur-state
                 (conj results {:task (:name task) :error "No operator" :ok false})))))))

;; ── Built-in operators ────────────────────────────────────────────────────────

(register-operator!
 (->Operator "ping"
             (fn [_] true)
             (fn [s _] {})
             (fn [_ p] (str "PONG from " (get p :host "localhost")))))

(register-operator!
 (->Operator "read_file"
             (fn [_] true)
             (fn [s p] {:last-file (:path p)})
             (fn [_ p] (try (slurp (:path p)) (catch Exception e (str "Error: " e))))))

(register-operator!
 (->Operator "write_file"
             (fn [_] true)
             (fn [s p] {:last-write (:path p)})
             (fn [_ p]
               (spit (:path p) (get p :content ""))
               (str "Written: " (:path p)))))

(register-operator!
 (->Operator "log_message"
             (fn [_] true)
             (fn [s _] {})
             (fn [_ p] (log/info "[Plan]" (:message p)) (:message p))))

;; ── Built-in methods ─────────────────────────────────────────────────────────

(register-method!
 (->Method "research"
           (fn [_ _] true)
           (fn [_ p]
             [(make-task "log_message" {:message (str "Researching: " (:query p))} true)
              (make-task "read_file"   {:path (or (:cache-file p) "/tmp/research_cache.txt")} true)])))

(register-method!
 (->Method "code_change"
           (fn [_ p] (contains? p :path))
           (fn [_ p]
             [(make-task "read_file"  {:path (:path p)} true)
              (make-task "write_file" {:path (:path p) :content (:new-content p "")} true)])))

(register-method!
 (->Method "setup_and_ping"
           (fn [_ _] true)
           (fn [_ p]
             [(make-task "log_message" {:message "Setting up..."} true)
              (make-task "ping" {:host (get p :host "localhost")} true)])))

;; ── Public helpers ────────────────────────────────────────────────────────────

(defn list-operators []  (keys @operators))
(defn list-methods   []  (keys @methods))
