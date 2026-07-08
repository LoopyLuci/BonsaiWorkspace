(ns omniharness.policy
  "Capability-based policy engine with rule compilation and evaluation.")

;; ── Rule types ──────────────────────────────────────────────────────────────
;; Each rule is {:condition fn :result keyword :priority int}
;; Result: :allow | :deny | :ask-user | :sandbox

(defn make-rule
  "Create a policy rule from a predicate and result."
  ([pred result] (make-rule pred result 0))
  ([pred result priority]
   {:condition pred :result result :priority priority}))

(defn compile-policy
  "Compile a flat list of [condition result] or rule maps into an evaluator fn.
   Returns (fn [action args session]) -> :allow | :deny | :ask-user | :sandbox"
  [rules]
  (let [sorted (sort-by :priority >
                         (map (fn [r]
                                (if (map? r) r
                                    (let [[pred res] r]
                                      (make-rule pred res 0))))
                              rules))]
    (fn [action args session]
      (or (some (fn [{:keys [condition result]}]
                  (when (condition action args session) result))
                sorted)
          :deny))))   ; default-deny

;; ── Built-in predicates ──────────────────────────────────────────────────────

(defn action-is? [name]
  (fn [action _ _] (= action name)))

(defn action-prefix? [prefix]
  (fn [action _ _] (clojure.string/starts-with? (str action) prefix)))

(defn has-session? []
  (fn [_ _ session] (some? session)))

(defn arg-matches? [key pattern]
  (fn [_ args _] (re-find (re-pattern pattern) (str (get args key "")))))

;; ── Default policy ───────────────────────────────────────────────────────────

(def default-policy
  (compile-policy
   [(make-rule (action-is? "ping")             :allow  100)
    (make-rule (action-is? "get_time")         :allow  100)
    (make-rule (action-is? "calculator")       :allow  100)
    (make-rule (action-is? "search_web")       :allow   90)
    (make-rule (action-is? "http_get")         :allow   80)
    (make-rule (action-is? "read_file")        :allow   70)
    (make-rule (action-prefix? "memory.")      :allow   70)
    (make-rule (action-is? "write_file")       :ask-user 60)
    (make-rule (action-is? "http_post")        :ask-user 60)
    (make-rule (action-prefix? "shell.")       :sandbox  50)
    (make-rule (fn [_ _ _] true)              :deny     0)]))

(defn evaluate
  "Evaluate a policy for an action. Returns :allow | :deny | :ask-user | :sandbox."
  ([action args]
   (evaluate action args nil))
  ([action args session]
   (default-policy action args session)))

(defn allowed? [action args]
  (= :allow (evaluate action args)))

(defn requires-approval? [action args]
  (= :ask-user (evaluate action args)))

;; ── Audit log ───────────────────────────────────────────────────────────────

(def ^:private audit-log (atom []))

(defn audit-decision! [action args result session-id]
  (swap! audit-log conj
         {:timestamp  (System/currentTimeMillis)
          :action     action
          :args       args
          :result     result
          :session-id session-id})
  result)

(defn get-audit-log [] @audit-log)

(defn clear-audit-log! [] (reset! audit-log []))
