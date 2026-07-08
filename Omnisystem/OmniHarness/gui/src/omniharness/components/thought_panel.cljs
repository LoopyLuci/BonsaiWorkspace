(ns omniharness.components.thought-panel
  "Chat message list with ReAct step visualization"
  (:require [reagent.core :as r]
            [re-frame.core :as rf]))

(defn- step-status-icon [status]
  (case status
    "success" "✓"
    "failed"  "✗"
    "running" "⟳"
    "○"))

(defn- step-card [{:keys [thought action action-input observation status latency-ms]}]
  (let [expanded? (r/atom false)]
    (fn [_]
      [:div.step-card {:class status}
       [:div.step-header {:on-click #(swap! expanded? not)}
        [:span.step-icon (step-status-icon status)]
        [:span.step-action action]
        (when latency-ms
          [:span.step-latency (str (js/Math.round latency-ms) "ms")])
        [:span.step-toggle (if @expanded? "▲" "▼")]]
       (when @expanded?
         [:div.step-detail
          (when (not (clojure.string/blank? thought))
            [:div.step-section
             [:span.step-label "Thought"]
             [:p thought]])
          (when (not (clojure.string/blank? action-input))
            [:div.step-section
             [:span.step-label "Input"]
             [:pre action-input]])
          (when (not (clojure.string/blank? observation))
            [:div.step-section
             [:span.step-label "Result"]
             [:pre observation]])])])))

(defn- message-bubble [{:keys [id role content steps model tokens timestamp]}]
  (let [msg-steps @(rf/subscribe [:agent-steps id])]
    [:div.message-row {:class role}
     [:div.message-bubble {:class (str role "-bubble")}
      [:p.message-content content]
      (when (seq msg-steps)
        [:div.steps-container
         (for [step msg-steps]
           ^{:key (:id step)}
           [step-card step])])
      (when (or model tokens)
        [:div.message-meta
         (when model  [:span.meta-model model])
         (when (and tokens (pos? tokens))
           [:span.meta-tokens (str tokens " tokens")])])]]))

(defn thought-panel []
  (let [messages @(rf/subscribe [:messages])
        loading? @(rf/subscribe [:is-loading])]
    [:div.thought-panel
     (if (empty? messages)
       [:div.empty-state
        [:div.empty-icon "⬡"]
        [:h2 "OmniHarness"]
        [:p "Your enterprise AI harness. Ask anything, run agents, search memory."]
        [:div.quick-actions
         [:button.quick-btn {:on-click #(rf/dispatch [:send-message "What can you do?"])}
          "What can you do?"]
         [:button.quick-btn {:on-click #(rf/dispatch [:send-message "List all available tools"])}
          "List tools"]
         [:button.quick-btn {:on-click #(rf/dispatch [:send-message "Show system status"])}
          "System status"]]]
       (for [msg messages]
         ^{:key (:id msg)}
         [message-bubble msg]))
     (when loading?
       [:div.loading-indicator
        [:div.pulse]
        [:span "Thinking…"]])]))
