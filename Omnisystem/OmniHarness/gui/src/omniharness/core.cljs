(ns omniharness.core
  "OmniHarness ClojureScript GUI — entry point"
  (:require [reagent.dom :as rdom]
            [re-frame.core :as rf]
            [omniharness.events]
            [omniharness.subs]
            [omniharness.views :refer [harness-shell]]))

(defn mount-root! []
  (rf/clear-subscription-cache!)
  (let [root-el (js/document.getElementById "app")]
    (rdom/render [harness-shell] root-el)))

(defn ^:export init! []
  (rf/dispatch-sync [:init])
  (rf/dispatch [:boot])
  (mount-root!))

;; Hot-reload hook (shadow-cljs)
(defn ^:dev/after-load after-load []
  (mount-root!))
