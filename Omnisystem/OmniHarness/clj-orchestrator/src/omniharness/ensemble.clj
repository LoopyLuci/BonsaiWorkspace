(ns omniharness.ensemble
  "Mixture-of-Agents / model ensemble — fan a prompt across many models and
   combine by concat, vote, judge synthesis, or layered MoA. Provider-agnostic:
   `llm-fn` is (fn [model messages system] -> future/string)."
  (:require [clojure.string :as str]
            [omniharness.governance :as gov]))

(defn- tokens-est [^String s] (max 1 (quot (count s) 4)))

(defn- ask
  "Call one model through the governor; returns its text (or an [error ...] marker)."
  [llm-fn governor model prompt system]
  (try
    (when governor (gov/check-model! governor model))
    (let [text (llm-fn model [{:role "user" :content prompt}] system)]
      (when governor (gov/record-call! governor model (+ (tokens-est prompt) (tokens-est text))))
      text)
    (catch Exception e (str "[error: " (.getMessage e) "]"))))

(defn- fan-out
  "Query all models concurrently (bounded by governor parallelism)."
  [llm-fn governor models prompt system]
  (let [futures (mapv (fn [m] [m (future (ask llm-fn governor m prompt system))]) models)]
    (into {} (map (fn [[m f]] [m @f]) futures))))

(defn- vote [answers]
  (let [norm  #(subs (str/lower-case (str/trim (str/replace % #"\s+" " "))) 0
                     (min 400 (count %)))
        freq  (frequencies (map norm (vals answers)))
        win   (key (apply max-key val freq))]
    (or (some (fn [a] (when (= (norm a) win) a)) (vals answers))
        (first (vals answers)))))

(defn- judge [llm-fn governor prompt system answers judge-model]
  (let [proposals (str/join "\n\n"
                            (map-indexed (fn [i [m t]] (str "[Candidate " (inc i) " — " m "]\n" t))
                                         answers))
        synth (str "You are an expert judge. The user asked:\n\n" prompt
                   "\n\nCandidate answers:\n\n" proposals
                   "\n\nSynthesize the single best, correct, complete answer.")]
    (ask llm-fn governor (or judge-model (ffirst answers)) synth system)))

(defn run-ensemble
  "Options: {:strategy :concat|:vote|:judge :judge-model m :system s :governor g}.
   Returns {:answers {model text} :final text :strategy s}."
  [llm-fn prompt models {:keys [strategy judge-model system governor]
                         :or   {strategy :judge}}]
  (when governor (gov/checkpoint! governor "ensemble"))
  (let [answers (fan-out llm-fn governor models prompt system)
        valid   (into {} (remove (fn [[_ t]] (str/starts-with? t "[error:")) answers))
        final   (case strategy
                  :concat (str/join "\n\n" (map (fn [[m t]] (str "### " m "\n" t)) valid))
                  :vote   (vote valid)
                  (judge llm-fn governor prompt system valid judge-model))]
    {:answers answers :final final :strategy strategy
     :usage (when governor (:usage @governor))}))

(defn layered-moa
  "Layered Mixture-of-Agents: proposers answer each layer using the prior layer's
   aggregated context; a final aggregator produces the answer."
  [llm-fn prompt proposers aggregator {:keys [layers system governor] :or {layers 2}}]
  (loop [layer 0, context ""]
    (if (< layer layers)
      (let [lp (if (str/blank? context) prompt
                   (str prompt "\n\nImprove upon:\n" context))
            props (fan-out llm-fn governor proposers lp system)]
        (recur (inc layer) (str/join "\n\n" (map #(str "- " %) (vals props)))))
      (ask llm-fn governor aggregator
           (str "Original request:\n" prompt "\n\nPanel proposals:\n" context
                "\n\nProduce the definitive final answer.")
           system))))
