(ns omniharness.components.memory-graph
  "Interactive memory graph — vector store visualization + search"
  (:require [reagent.core :as r]
            [re-frame.core :as rf]))

(defn- memory-node [{:keys [id content score importance x y]} selected?]
  [:div.memory-node
   {:style    {:left (str (* x 100) "%")
               :top  (str (* y 100) "%")}
    :class    (cond
                selected?                 "selected"
                (and importance
                     (> importance 0.7))  "hot"
                :else                     "")
    :title    (str content "\nScore: " (when score (js/Math.round (* score 100))) "%")}
   [:span.node-label
    (let [words (clojure.string/split content #"\s+")]
      (clojure.string/join " " (take 4 words)))]])

(defn memory-graph []
  (let [results    @(rf/subscribe [:memory-results])
        search-q   (r/atom "")
        selected   (r/atom nil)
        is-loading @(rf/subscribe [:is-loading])]
    (fn []
      ;; Assign stable positions based on score/index
      (let [positioned (map-indexed
                         (fn [i m]
                           (let [angle (* i (/ (* 2 js/Math.PI) (max 1 (count results))))
                                 r     (+ 0.25 (* 0.25 (or (:score m) 0.5)))]
                             (assoc m
                                    :x (+ 0.5 (* r (js/Math.cos angle)))
                                    :y (+ 0.5 (* r (js/Math.sin angle))))))
                         results)]
        [:div.memory-panel
         [:div.memory-search-row
          [:input.memory-search-input
           {:type        "text"
            :placeholder "Search memory…"
            :value       @search-q
            :on-change   #(reset! search-q (-> % .-target .-value))
            :on-key-down (fn [e]
                           (when (= (.-key e) "Enter")
                             (rf/dispatch [:memory-search @search-q])))}]
          [:button.btn-search
           {:on-click #(rf/dispatch [:memory-search @search-q])
            :disabled is-loading}
           "Search"]]
         (if (empty? results)
           [:div.memory-empty
            [:span "No memories yet. Memories are stored automatically during conversations."]]
           [:div.memory-graph-container
            ;; SVG edges between related nodes (simplified: ring connections)
            [:svg.memory-edges {:width "100%" :height "100%"}
             (for [[a b] (partition 2 1 (concat positioned [(first positioned)]))]
               (when (and a b)
                 ^{:key (str (:id a) "-" (:id b))}
                 [:line {:x1            (str (* (:x a) 100) "%")
                          :y1            (str (* (:y a) 100) "%")
                          :x2            (str (* (:x b) 100) "%")
                          :y2            (str (* (:y b) 100) "%")
                          :stroke        "#2a2a38"
                          :stroke-width  "1"}]))]
            ;; Nodes
            (for [node positioned]
              ^{:key (:id node)}
              [memory-node node (= @selected (:id node))])])
         (when @selected
           (let [mem (first (filter #(= (:id %) @selected) results))]
             (when mem
               [:div.memory-detail
                [:p.memory-detail-content (:content mem)]
                [:div.memory-detail-meta
                 [:span (str "Score: " (when (:score mem)
                                         (str (js/Math.round (* (:score mem) 100)) "%")))]]])))]))))
