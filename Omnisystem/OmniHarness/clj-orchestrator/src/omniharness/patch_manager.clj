(ns omniharness.patch-manager
  "Self-evolution patch manager — proposes, reviews, and applies patches."
  (:require [clojure.java.io    :as io]
            [cheshire.core      :as json]
            [mount.core         :refer [defstate]]
            [taoensso.timbre    :as log]
            [omniharness.events :as events]))

;; ── Patch proposal model ──────────────────────────────────────────────────────

(defrecord PatchProposal
  [proposal-id
   description
   target-file
   patch-content    ; unified diff or full replacement
   status           ; :pending | :approved | :rejected | :applied
   proposed-at
   reviewed-at
   applied-at])

(defn new-proposal [description target-file patch-content]
  (->PatchProposal
   (str (java.util.UUID/randomUUID))
   description
   target-file
   patch-content
   :pending
   (System/currentTimeMillis)
   nil
   nil))

;; ── In-memory proposal store ──────────────────────────────────────────────────

(def ^:private proposals (atom {}))

(defn submit-proposal! [description target-file patch-content]
  (let [p (new-proposal description target-file patch-content)]
    (swap! proposals assoc (:proposal-id p) p)
    (events/append-event! "patch-manager" "ProposalSubmitted"
                          {:id (:proposal-id p) :target target-file})
    (log/info "[PatchManager] Proposal submitted:" (:proposal-id p) "-" description)
    p))

(defn approve-proposal! [proposal-id]
  (if-let [p (get @proposals proposal-id)]
    (do (swap! proposals update proposal-id assoc :status :approved :reviewed-at (System/currentTimeMillis))
        (events/append-event! "patch-manager" "ProposalApproved" {:id proposal-id})
        (log/info "[PatchManager] Approved:" proposal-id)
        true)
    (do (log/warn "[PatchManager] Unknown proposal:" proposal-id) false)))

(defn reject-proposal! [proposal-id reason]
  (if (get @proposals proposal-id)
    (do (swap! proposals update proposal-id assoc :status :rejected
               :reviewed-at (System/currentTimeMillis)
               :rejection-reason reason)
        (events/append-event! "patch-manager" "ProposalRejected" {:id proposal-id :reason reason})
        (log/info "[PatchManager] Rejected:" proposal-id)
        true)
    false))

(defn apply-patch! [proposal-id]
  (if-let [p (get @proposals proposal-id)]
    (if (= :approved (:status p))
      (do
        ;; Write patch content to target file
        (try
          (spit (:target-file p) (:patch-content p))
          (swap! proposals update proposal-id assoc
                 :status :applied :applied-at (System/currentTimeMillis))
          (events/append-event! "patch-manager" "PatchApplied"
                                {:id proposal-id :target (:target-file p)})
          (log/info "[PatchManager] Applied patch to:" (:target-file p))
          {:ok true :target (:target-file p)}
          (catch Exception e
            (log/error "[PatchManager] Apply failed:" (.getMessage e))
            {:ok false :error (.getMessage e)})))
      {:ok false :error (str "Proposal not approved. Status: " (:status p))})
    {:ok false :error "Proposal not found"}))

(defn list-proposals
  ([] (vals @proposals))
  ([status-filter] (filter #(= status-filter (:status %)) (vals @proposals))))

(defn get-proposal [id] (get @proposals id))

;; ── Watcher loop ──────────────────────────────────────────────────────────────

(defstate proposal-watcher
  :start (future
           (loop []
             (Thread/sleep 10000)
             (let [pending (list-proposals :pending)]
               (when (seq pending)
                 (log/info "[PatchManager] Pending proposals:" (count pending)
                           (map :proposal-id pending))))
             (recur)))
  :stop (future-cancel proposal-watcher))
