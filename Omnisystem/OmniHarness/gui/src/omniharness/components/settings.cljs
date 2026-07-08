(ns omniharness.components.settings
  "Settings panel — model defaults, API keys, behavior"
  (:require [re-frame.core :as rf]))

(defn- toggle [{:keys [label desc setting-key]}]
  (let [value @(rf/subscribe [:setting setting-key])]
    [:div.settings-row
     [:div.settings-label
      [:span.settings-row-label label]
      [:span.settings-row-desc desc]]
     [:button.toggle-btn {:class    (when value "on")
                          :on-click #(rf/dispatch [:update-setting setting-key (not value)])}
      (if value "ON" "OFF")]]))

(defn- slider [{:keys [label desc setting-key min-val max-val step format-fn]}]
  (let [value @(rf/subscribe [:setting setting-key])]
    [:div.settings-row
     [:div.settings-label
      [:span.settings-row-label label]
      [:span.settings-row-desc desc]]
     [:div.slider-group
      [:input {:type      "range"
               :min       min-val
               :max       max-val
               :step      step
               :value     value
               :on-change #(rf/dispatch [:update-setting setting-key
                                         (js/parseFloat (-> % .-target .-value))])}]
      [:span.slider-value ((or format-fn str) value)]]]))

(defn settings []
  (let [token-total @(rf/subscribe [:token-total])
        session-id  @(rf/subscribe [:session-id])]
    [:div.settings-panel
     [:h2.settings-title "Settings"]

     [:div.settings-section
      [:h3.settings-section-title "Model Behavior"]
      [slider {:label       "Temperature"
               :desc        "Higher = more creative, lower = more focused"
               :setting-key :temperature
               :min-val     0
               :max-val     2
               :step        0.05
               :format-fn   #(str (.toFixed % 2))}]
      [slider {:label       "Max Tokens"
               :desc        "Maximum response length"
               :setting-key :max-tokens
               :min-val     256
               :max-val     32000
               :step        256
               :format-fn   #(str % " tokens")}]]

     [:div.settings-section
      [:h3.settings-section-title "Features"]
      [toggle {:label       "Streaming"
               :desc        "Stream responses token-by-token"
               :setting-key :use-streaming}]
      [toggle {:label       "Agent Mode"
               :desc        "Use ReAct agent for multi-step reasoning"
               :setting-key :use-agent-mode}]
      [toggle {:label       "Memory"
               :desc        "Automatically store and retrieve conversation context"
               :setting-key :memory-enabled}]]

     [:div.settings-section
      [:h3.settings-section-title "Session Info"]
      [:div.settings-info-row
       [:span.info-label "Session ID"]
       [:span.info-value.mono (or session-id "—")]]
      [:div.settings-info-row
       [:span.info-label "Total Tokens"]
       [:span.info-value (str token-total)]]
      [:button.btn-danger
       {:on-click #(rf/dispatch [:new-session])}
       "Start New Session"]]]))
