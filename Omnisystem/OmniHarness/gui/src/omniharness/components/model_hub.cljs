(ns omniharness.components.model-hub
  "Model selector with provider grouping and health indicators"
  (:require [reagent.core :as r]
            [re-frame.core :as rf]))

(defn- provider-color [provider]
  (case provider
    "anthropic"  "#cc8855"
    "openai"     "#10a37f"
    "google"     "#4285f4"
    "cohere"     "#d4a017"
    "mistral"    "#ff7000"
    "groq"       "#f55036"
    "openrouter" "#9d4edd"
    "ollama"     "#4ecca3"
    "#888899"))

(defn- model-option [{:keys [id display_name provider context_window supports_tools available]}]
  (let [current @(rf/subscribe [:model-id])]
    [:div.model-option {:class    (when (= id current) "selected")
                        :on-click #(rf/dispatch [:select-model id])}
     [:div.model-option-left
      [:div.provider-dot {:style {:background (provider-color provider)}}]
      [:div.model-option-info
       [:span.model-option-name display_name]
       [:span.model-option-provider provider]]]
     [:div.model-option-badges
      (when supports_tools [:span.badge.tools "tools"])
      (when (and context_window (pos? context_window))
        [:span.badge.ctx (str (quot context_window 1000) "k")])
      (when-not available [:span.badge.offline "offline"])]]))

(defn model-hub []
  (let [models   @(rf/subscribe [:models])
        current  @(rf/subscribe [:current-model-info])
        search   (r/atom "")
        open?    (r/atom false)]
    (fn []
      (let [filtered (if (clojure.string/blank? @search)
                       models
                       (filter #(or (clojure.string/includes?
                                     (clojure.string/lower-case (:display_name % ""))
                                     (clojure.string/lower-case @search))
                                    (clojure.string/includes?
                                     (clojure.string/lower-case (:provider % ""))
                                     (clojure.string/lower-case @search)))
                               models))]
        [:div.model-selector
         [:div.model-current {:class    (when @open? "open")
                               :on-click #(swap! open? not)}
          [:div.provider-dot {:style {:background (provider-color (:provider current))}}]
          [:span.model-name (:display_name current (:id current))]
          [:span.chevron (if @open? "▲" "▼")]]
         (when @open?
           [:div.model-dropdown
            [:input.model-search
             {:type        "text"
              :placeholder "Search models…"
              :value       @search
              :auto-focus  true
              :on-change   #(reset! search (-> % .-target .-value))}]
            (if (empty? filtered)
              [:div.model-empty "No models found"]
              (for [m filtered]
                ^{:key (:id m)}
                [model-option m]))])]))))
