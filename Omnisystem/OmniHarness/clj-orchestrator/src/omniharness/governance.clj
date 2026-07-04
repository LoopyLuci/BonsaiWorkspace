(ns omniharness.governance
  "Substrate governance — resource budgets, capability policy, a hash-chained
   audit log, and a kill switch. Pure, data-oriented, and thread-safe via atoms."
  (:require [clojure.string :as str])
  (:import [java.security MessageDigest]))

;; ── Audit hash chain ─────────────────────────────────────────────────────────

(defn- sha256 [^String s]
  (let [d (.digest (MessageDigest/getInstance "SHA-256") (.getBytes s "UTF-8"))]
    (apply str (map #(format "%02x" (bit-and % 0xff)) d))))

(def ^:private zero-hash (apply str (repeat 64 "0")))

(defn new-audit [] (atom {:events [] :head zero-hash}))

(defn audit-append!
  "Append a tamper-evident event; returns the new head hash."
  [audit kind payload]
  (let [ts     (System/currentTimeMillis)
        head   (:head @audit)
        body   (pr-str payload)
        digest (sha256 (str head ts kind body))]
    (swap! audit (fn [a]
                   (-> a
                       (update :events conj {:seq (count (:events a)) :ts ts
                                             :kind kind :payload payload
                                             :prev head :hash digest})
                       (assoc :head digest))))
    digest))

(defn audit-verify?
  "Replay the chain; true iff no event was mutated or reordered."
  [audit]
  (loop [head zero-hash, evs (:events @audit)]
    (if-let [ev (first evs)]
      (let [digest (sha256 (str head (:ts ev) (:kind ev) (pr-str (:payload ev))))]
        (if (and (= digest (:hash ev)) (= (:prev ev) head))
          (recur digest (rest evs))
          false))
      true)))

;; ── Budgets, policy, kill switch, governor ───────────────────────────────────

(def default-budget
  {:max-model-calls 200 :max-tokens 2000000 :max-cost-usd 10.0
   :max-steps 500 :max-wallclock-ms 1800000 :max-parallel 16})

(def default-policy
  {:allowed-models #{} :allowed-tools #{} :denied-tools #{}
   :allow-network true :allow-fs-write true :max-agents 64})

(def ^:private cost-per-1k 0.005)

(defn new-governor
  ([] (new-governor default-budget default-policy))
  ([budget policy]
   (let [g {:budget budget :policy policy
            :usage  {:model-calls 0 :tokens 0 :cost-usd 0.0 :steps 0
                     :started-ms (System/currentTimeMillis)}
            :kill   {:tripped false :reason ""}
            :audit  (new-audit)}]
     (audit-append! (:audit g) "run_start" {})
     (atom g))))

(defn trip! [gov reason]
  (swap! gov assoc :kill {:tripped true :reason reason}))

(defn checkpoint!
  "Enforce kill switch, step, and wall-clock limits. Throws ex-info on violation."
  [gov note]
  (let [{:keys [kill budget usage audit]} @gov]
    (when (:tripped kill)
      (audit-append! audit "aborted" {:reason (:reason kill)})
      (throw (ex-info (:reason kill) {:type :aborted})))
    (swap! gov update-in [:usage :steps] inc)
    (let [steps (get-in @gov [:usage :steps])
          elapsed (- (System/currentTimeMillis) (:started-ms usage))]
      (when (and (pos? (:max-steps budget)) (> steps (:max-steps budget)))
        (throw (ex-info (str "max-steps exceeded at " note) {:type :budget})))
      (when (and (pos? (:max-wallclock-ms budget)) (> elapsed (:max-wallclock-ms budget)))
        (throw (ex-info "max-wallclock-ms exceeded" {:type :budget}))))))

(defn check-model! [gov model]
  (let [{:keys [policy budget usage audit]} @gov
        allowed (:allowed-models policy)]
    (when (and (seq allowed) (not (contains? allowed model)))
      (audit-append! audit "policy_violation" {:model model})
      (throw (ex-info (str "model denied: " model) {:type :policy})))
    (when (and (pos? (:max-model-calls budget))
               (>= (:model-calls usage) (:max-model-calls budget)))
      (throw (ex-info "max-model-calls exceeded" {:type :budget})))))

(defn check-tool! [gov tool]
  (let [{:keys [policy audit]} @gov]
    (when (or (contains? (:denied-tools policy) tool)
              (and (seq (:allowed-tools policy))
                   (not (contains? (:allowed-tools policy) tool))))
      (audit-append! audit "policy_violation" {:tool tool})
      (throw (ex-info (str "tool denied: " tool) {:type :policy})))))

(defn record-call! [gov model tokens]
  (swap! gov (fn [g]
               (-> g
                   (update-in [:usage :model-calls] inc)
                   (update-in [:usage :tokens] + tokens)
                   (update-in [:usage :cost-usd] + (* (/ tokens 1000.0) cost-per-1k)))))
  (audit-append! (:audit @gov) "model_call" {:model model :tokens tokens})
  (let [{:keys [budget usage]} @gov]
    (when (and (pos? (:max-tokens budget)) (> (:tokens usage) (:max-tokens budget)))
      (throw (ex-info "max-tokens exceeded" {:type :budget})))
    (when (and (pos? (:max-cost-usd budget)) (> (:cost-usd usage) (:max-cost-usd budget)))
      (throw (ex-info "max-cost-usd exceeded" {:type :budget})))))

(defn parallelism [gov requested]
  (let [{:keys [budget policy]} @gov]
    (max 1 (min requested (:max-parallel budget)
                (if (pos? (:max-agents policy)) (:max-agents policy) requested)))))

(defn report [gov]
  (let [{:keys [usage budget audit kill]} @gov]
    {:usage usage :budget budget
     :audit-valid (audit-verify? audit)
     :audit-events (count (:events @audit))
     :killed (:tripped kill)}))
