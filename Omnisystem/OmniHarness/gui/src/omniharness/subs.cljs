(ns omniharness.subs
  "Re-frame subscriptions — derived views of application state"
  (:require [re-frame.core :as rf]))

(rf/reg-sub :session-id     (fn [db _] (:session-id db)))
(rf/reg-sub :model-id       (fn [db _] (:model-id db)))
(rf/reg-sub :messages       (fn [db _] (:messages db)))
(rf/reg-sub :sessions       (fn [db _] (:sessions db)))
(rf/reg-sub :models         (fn [db _] (:models db)))
(rf/reg-sub :tools          (fn [db _] (:tools db)))
(rf/reg-sub :memory-results (fn [db _] (:memory-results db)))
(rf/reg-sub :is-loading     (fn [db _] (:is-loading db)))
(rf/reg-sub :sidebar-open   (fn [db _] (:sidebar-open db)))
(rf/reg-sub :active-panel   (fn [db _] (:active-panel db)))
(rf/reg-sub :token-total    (fn [db _] (:token-total db)))
(rf/reg-sub :error          (fn [db _] (:error db)))
(rf/reg-sub :settings       (fn [db _] (:settings db)))

(rf/reg-sub :agent-steps
  (fn [db [_ msg-id]]
    (get-in db [:agent-steps msg-id] [])))

(rf/reg-sub :message-count
  :<- [:messages]
  (fn [msgs _] (count msgs)))

(rf/reg-sub :current-model-info
  :<- [:models]
  :<- [:model-id]
  (fn [[models model-id] _]
    (or (first (filter #(= (:id %) model-id) models))
        {:id model-id :display_name model-id :provider "unknown"})))

(rf/reg-sub :setting
  :<- [:settings]
  (fn [settings [_ k]]
    (get settings k)))

(rf/reg-sub :session-id-short
  :<- [:session-id]
  (fn [sid _]
    (when sid (subs sid 0 (min 8 (count sid))))))
