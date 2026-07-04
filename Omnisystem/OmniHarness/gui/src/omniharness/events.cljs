(ns omniharness.events
  "Re-frame event handlers — all state mutations go through here"
  (:require [re-frame.core :as rf]
            [cljs.core.async :refer [go <!]]
            [omniharness.client :as api]))

;; ── DB shape ──────────────────────────────────────────────────────────────────

(def default-db
  {:session-id       nil
   :model-id         "claude-sonnet-4-6"
   :messages         []
   :agent-steps      {}      ;; msg-id → [steps]
   :sessions         []
   :models           []
   :tools            []
   :memory-results   []
   :is-loading       false
   :is-streaming     false
   :sidebar-open     true
   :active-panel     :chat   ;; :chat | :memory | :tools | :settings
   :token-total      0
   :error            nil
   :ws-conn          nil
   :settings         {:temperature    0.7
                      :max-tokens     4096
                      :use-streaming  true
                      :use-agent-mode false
                      :memory-enabled true
                      :theme          :dark}})

;; ── Init ──────────────────────────────────────────────────────────────────────

(rf/reg-event-db :init
  (fn [_ _] default-db))

(rf/reg-event-fx :boot
  (fn [{:keys [db]} _]
    {:db (assoc db :is-loading true)
     :dispatch-n [[:load-models] [:load-tools] [:create-session]]}))

;; ── Sessions ──────────────────────────────────────────────────────────────────

(rf/reg-event-fx :create-session
  (fn [{:keys [db]} _]
    {:db db
     :async-dispatch
     (go (let [result (<! (api/create-session! (:model-id db)))]
           (if (:ok result)
             [:session-created (get-in result [:data :session_id])]
             [:set-error "Failed to create session"])))}))

(rf/reg-event-db :session-created
  (fn [db [_ sid]]
    (-> db
        (assoc :session-id sid)
        (assoc :is-loading false)
        (assoc :messages []))))

(rf/reg-event-fx :new-session
  (fn [{:keys [db]} _]
    (let [old-ws (:ws-conn db)]
      (when old-ws (.close old-ws))
      {:db (assoc db :messages [] :token-total 0 :ws-conn nil :error nil)
       :dispatch [:create-session]})))

;; ── Models ────────────────────────────────────────────────────────────────────

(rf/reg-event-fx :load-models
  (fn [{:keys [db]} _]
    {:db db
     :async-dispatch
     (go (let [result (<! (api/list-models!))]
           (if (:ok result)
             [:models-loaded (get-in result [:data :models] [])]
             [:set-error "Failed to load models"])))}))

(rf/reg-event-db :models-loaded
  (fn [db [_ models]] (assoc db :models models)))

(rf/reg-event-db :select-model
  (fn [db [_ model-id]]
    (assoc db :model-id model-id)))

;; ── Tools ─────────────────────────────────────────────────────────────────────

(rf/reg-event-fx :load-tools
  (fn [{:keys [db]} _]
    {:db db
     :async-dispatch
     (go (let [result (<! (api/list-tools!))]
           (if (:ok result)
             [:tools-loaded (get-in result [:data :tools] [])]
             [:set-error "Failed to load tools"])))}))

(rf/reg-event-db :tools-loaded
  (fn [db [_ tools]] (assoc db :tools tools)))

;; ── Chat ──────────────────────────────────────────────────────────────────────

(rf/reg-event-fx :send-message
  (fn [{:keys [db]} [_ text]]
    (let [msg-id  (str "msg-" (js/Date.now))
          user-msg {:id msg-id :role "user" :content text
                    :timestamp (js/Date.now)}]
      {:db         (-> db
                       (update :messages conj user-msg)
                       (assoc :is-loading true)
                       (assoc :error nil))
       :dispatch   (if (get-in db [:settings :use-agent-mode])
                     [:run-agent text]
                     [:chat-request text])})))

(rf/reg-event-fx :chat-request
  (fn [{:keys [db]} [_ text]]
    {:db db
     :async-dispatch
     (go (let [messages (->> (:messages db)
                             (map #(select-keys % [:role :content]))
                             (mapv #(clojure.set/rename-keys % {})))
               result   (<! (api/chat!
                              {:model-id   (:model-id db)
                               :messages   messages
                               :temperature (get-in db [:settings :temperature])
                               :max-tokens  (get-in db [:settings :max-tokens])
                               :session-id  (:session-id db)}))]
           (if (:ok result)
             [:chat-response (:data result)]
             [:set-error (str "Chat failed: " (:error result) )])))}))

(rf/reg-event-db :chat-response
  (fn [db [_ resp]]
    (let [msg {:id        (str "msg-" (js/Date.now))
               :role      "assistant"
               :content   (:content resp)
               :model     (:model_used resp)
               :tokens    (+ (get resp :input_tokens 0)
                             (get resp :output_tokens 0))
               :timestamp (js/Date.now)}]
      (-> db
          (update :messages conj msg)
          (update :token-total + (:tokens msg 0))
          (assoc :is-loading false)))))

;; ── Agent ─────────────────────────────────────────────────────────────────────

(rf/reg-event-fx :run-agent
  (fn [{:keys [db]} [_ objective]]
    {:db db
     :async-dispatch
     (go (let [result (<! (api/agent-run!
                            {:objective  objective
                             :model-id   (:model-id db)
                             :max-steps  20
                             :session-id (:session-id db)}))]
           (if (:ok result)
             [:agent-response (:data result)]
             [:set-error (str "Agent failed: " (:error result))])))}))

(rf/reg-event-db :agent-response
  (fn [db [_ resp]]
    (let [msg-id (str "msg-" (js/Date.now))
          msg    {:id        msg-id
                  :role      "assistant"
                  :content   (:answer resp)
                  :steps     (:steps resp [])
                  :model     (:model_id resp)
                  :tokens    0
                  :timestamp (js/Date.now)}]
      (-> db
          (update :messages conj msg)
          (assoc-in [:agent-steps msg-id] (:steps resp []))
          (assoc :is-loading false)))))

;; ── Memory ────────────────────────────────────────────────────────────────────

(rf/reg-event-fx :memory-search
  (fn [{:keys [db]} [_ query]]
    {:db (assoc db :is-loading true)
     :async-dispatch
     (go (let [result (<! (api/memory-search!
                            {:collection "default"
                             :query      query
                             :top-k      10}))]
           (if (:ok result)
             [:memory-results-loaded (get-in result [:data :results] [])]
             [:set-error "Memory search failed"])))}))

(rf/reg-event-db :memory-results-loaded
  (fn [db [_ results]]
    (-> db
        (assoc :memory-results results)
        (assoc :is-loading false))))

;; ── UI state ──────────────────────────────────────────────────────────────────

(rf/reg-event-db :toggle-sidebar
  (fn [db _] (update db :sidebar-open not)))

(rf/reg-event-db :set-panel
  (fn [db [_ panel]] (assoc db :active-panel panel)))

(rf/reg-event-db :set-error
  (fn [db [_ msg]]
    (-> db
        (assoc :error msg)
        (assoc :is-loading false))))

(rf/reg-event-db :clear-error
  (fn [db _] (assoc db :error nil)))

(rf/reg-event-db :update-setting
  (fn [db [_ k v]]
    (assoc-in db [:settings k] v)))

;; ── Async effect ─────────────────────────────────────────────────────────────

(rf/reg-fx :async-dispatch
  (fn [chan]
    (go (let [event (<! chan)]
          (rf/dispatch event)))))
