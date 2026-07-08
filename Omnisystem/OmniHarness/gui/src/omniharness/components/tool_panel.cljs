(ns omniharness.components.tool-panel
  "Tool browser and manual execution panel"
  (:require [reagent.core :as r]
            [re-frame.core :as rf]
            [cljs.core.async :refer [go <!]]
            [omniharness.client :as api]))

(defn- tool-badge [category]
  [:span.badge {:class (case category
                          "file"    "badge-file"
                          "web"     "badge-web"
                          "compute" "badge-compute"
                          "system"  "badge-system"
                          "badge-default")}
   category])

(defn- tool-entry [{:keys [name description category parameters]}]
  (let [expanded? (r/atom false)
        args      (r/atom "{}")
        result    (r/atom nil)
        running?  (r/atom false)]
    (fn [_]
      [:div.tool-entry {:class (when @running? "active")}
       [:div.tool-header {:on-click #(swap! expanded? not)}
        [:span.tool-name name]
        [tool-badge (or category "tool")]
        [:span.tool-toggle (if @expanded? "▲" "▼")]]
       (when @expanded?
         [:div.tool-detail
          [:p.tool-desc description]
          (when (seq parameters)
            [:div.tool-params
             [:span.tool-label "Parameters"]
             (for [[param-name param-info] parameters]
               ^{:key param-name}
               [:div.param-row
                [:span.param-name (str param-name)]
                [:span.param-type (get param-info "type" "any")]
                (when (get param-info "required")
                  [:span.param-required "*"])])])
          [:div.tool-run
           [:textarea.tool-args-input
            {:value     @args
             :rows      3
             :on-change #(reset! args (-> % .-target .-value))
             :placeholder "{\"param\": \"value\"}"}]
           [:button.btn-run
            {:disabled @running?
             :on-click (fn [_]
                         (reset! running? true)
                         (reset! result nil)
                         (go (let [parsed  (try (js->clj (js/JSON.parse @args))
                                                (catch :default _ {}))
                                   res     (<! (api/execute-tool!
                                                 {:name name :arguments parsed}))]
                               (reset! result res)
                               (reset! running? false))))}
            (if @running? "Running…" "Run Tool")]]
          (when @result
            [:div.tool-result
             [:span.tool-label (if (:ok @result) "Result" "Error")]
             [:pre {:class (if (:ok @result) "" "error")}
              (if (:ok @result)
                (-> @result :data :result str)
                (:error @result))]])])])))

(defn tool-panel []
  (let [tools   @(rf/subscribe [:tools])
        filter  (r/atom "")]
    (fn []
      (let [visible (if (clojure.string/blank? @filter)
                      tools
                      (filter #(clojure.string/includes?
                                 (clojure.string/lower-case (:name % ""))
                                 (clojure.string/lower-case @filter))
                              tools))]
        [:div.tool-panel
         [:div.tool-filter-row
          [:input.tool-filter
           {:type        "text"
            :placeholder "Filter tools…"
            :value       @filter
            :on-change   #(reset! filter (-> % .-target .-value))}]
          [:span.tool-count (str (count visible) " tools")]]
         (if (empty? tools)
           [:div.tool-empty
            [:span "Loading tools…"]]
           [:div.tool-list
            (for [tool visible]
              ^{:key (:name tool)}
              [tool-entry tool])])]))))
