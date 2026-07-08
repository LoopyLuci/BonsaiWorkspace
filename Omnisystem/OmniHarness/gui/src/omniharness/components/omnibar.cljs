(ns omniharness.components.omnibar
  "The primary chat input — the OmniBar"
  (:require [reagent.core :as r]
            [re-frame.core :as rf]))

(defn omnibar []
  (let [text     (r/atom "")
        loading? @(rf/subscribe [:is-loading])]
    (fn []
      [:div.omnibar-container
       [:div.omnibar-inner {:class (when loading? "disabled")}
        [:textarea.omnibar-input
         {:placeholder  "Ask anything, or @ a tool… (Enter to send, Shift+Enter newline)"
          :value        @text
          :disabled     loading?
          :rows         1
          :on-change    #(reset! text (-> % .-target .-value))
          :on-key-down  (fn [e]
                          (when (and (= (.-key e) "Enter")
                                     (not (.-shiftKey e))
                                     (not loading?))
                            (.preventDefault e)
                            (when-not (clojure.string/blank? @text)
                              (rf/dispatch [:send-message @text])
                              (reset! text ""))))}]
        [:div.omnibar-actions
         [:button.btn-icon {:title    "Attach file"
                             :disabled loading?}
          [:span "📎"]]
         [:button.btn-send
          {:class    (when loading? "disabled")
           :disabled loading?
           :on-click (fn [_]
                       (when-not (or loading? (clojure.string/blank? @text))
                         (rf/dispatch [:send-message @text])
                         (reset! text "")))}
          (if loading?
            [:span.spinner "⟳"]
            [:span "↑"])]]]])))
