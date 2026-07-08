(ns omniharness.views
  "Top-level view — assembles all panels into the full shell layout"
  (:require [re-frame.core :as rf]
            [omniharness.components.omnibar       :refer [omnibar]]
            [omniharness.components.thought-panel :refer [thought-panel]]
            [omniharness.components.model-hub     :refer [model-hub]]
            [omniharness.components.memory-graph  :refer [memory-graph]]
            [omniharness.components.tool-panel    :refer [tool-panel]]
            [omniharness.components.settings      :refer [settings]]))

;; ── Sidebar ───────────────────────────────────────────────────────────────────

(defn sidebar []
  (let [open?        @(rf/subscribe [:sidebar-open])
        active-panel @(rf/subscribe [:active-panel])
        token-total  @(rf/subscribe [:token-total])
        session-id   @(rf/subscribe [:session-id-short])]
    [:div.sidebar {:class (if open? "open" "closed")}
     [:div.sidebar-header
      [:span.logo "⬡ OmniHarness"]
      [:button.btn-icon {:on-click #(rf/dispatch [:toggle-sidebar])} "◀"]]

     ;; Model selector
     [model-hub]

     ;; Navigation
     [:nav.sidebar-nav
      (for [[panel icon label] [[:chat    "💬" "Chat"]
                                 [:memory  "🧠" "Memory"]
                                 [:tools   "🔧" "Tools"]
                                 [:settings "⚙" "Settings"]]]
        ^{:key panel}
        [:button.nav-btn {:class    (when (= active-panel panel) "active")
                          :on-click #(rf/dispatch [:set-panel panel])}
         [:span.nav-icon icon]
         [:span.nav-label label]])]

     ;; Footer stats
     [:div.sidebar-footer
      [:div.stat-row
       [:span "Tokens used"]
       [:span.stat-value (str token-total)]]
      (when session-id
        [:div.stat-row
         [:span "Session"]
         [:span.stat-value.mono session-id]])]]))

;; ── Topbar ────────────────────────────────────────────────────────────────────

(defn topbar []
  (let [sidebar-open? @(rf/subscribe [:sidebar-open])
        active-panel  @(rf/subscribe [:active-panel])
        error         @(rf/subscribe [:error])]
    [:div.topbar
     (when-not sidebar-open?
       [:button.btn-icon {:on-click #(rf/dispatch [:toggle-sidebar])} "☰"])
     [:span.panel-title
      (case active-panel
        :chat     "Chat"
        :memory   "Memory Graph"
        :tools    "Tool Browser"
        :settings "Settings"
        "OmniHarness")]
     [:div.topbar-right
      [:button.btn-new-session {:on-click #(rf/dispatch [:new-session])}
       "New Session"]]
     (when error
       [:div.error-banner
        [:span error]
        [:button {:on-click #(rf/dispatch [:clear-error])} "✕"]])]))

;; ── Main panel content ────────────────────────────────────────────────────────

(defn main-panel []
  (let [active @(rf/subscribe [:active-panel])]
    [:div.main-content
     (case active
       :chat     [:<> [thought-panel]
                      [:div.input-area [omnibar]]]
       :memory   [memory-graph]
       :tools    [tool-panel]
       :settings [settings]
       [thought-panel])]))

;; ── Root shell ────────────────────────────────────────────────────────────────

(defn harness-shell []
  [:div.harness-shell
   [sidebar]
   [:div.main-area
    [topbar]
    [main-panel]]])
