(ns omniharness.core
  "OmniHarness Clojure Orchestrator entry point."
  (:require [mount.core          :as mount]
            [clojure.core.async  :refer [<!!]]
            [cheshire.core       :as json]
            [taoensso.timbre     :as log]
            [omniharness.client  :refer [channel event-store-stub model-stub
                                         memory-stub tool-stub session-stub harness-stub]]
            [omniharness.events  :as events]
            [omniharness.policy  :as policy]
            [omniharness.react-engine :as react]
            [omniharness.planner :as planner]
            [omniharness.patch-manager :as patches]
            ;; Registers the http-server mount state (started by boot! via
            ;; mount/start below) — required here so `serve` actually has an
            ;; HTTP API for orchestrator/omniharness/clj_client.py to call.
            [omniharness.http-server])
  (:gen-class))

(defn boot! []
  (log/info "═══════════════════════════════════════════════════════")
  (log/info "  OmniHarness Clojure Orchestrator v1.0.0              ")
  (log/info "═══════════════════════════════════════════════════════")
  (mount/start))

(defn shutdown! []
  (log/info "[Shutdown] Stopping Clojure orchestrator...")
  (mount/stop))

(defn run-agent!
  "Run a ReAct agent for an objective. Blocks until done."
  [objective & {:keys [model-id max-steps]
                :or   {model-id "claude-sonnet-4-6" max-steps 20}}]
  (log/info "[Agent] Starting ReAct loop for:" objective)
  (let [result (<!! (react/run-react-loop
                     {:objective objective
                      :model-id  model-id
                      :max-steps max-steps}))]
    (log/info "[Agent] Done. Success:" (:success result))
    (log/info "[Agent] Answer:" (:answer result))
    result))

(defn verify-kernel! []
  (try
    (let [chain (events/verify-chain!)]
      (if (:valid chain)
        (log/info "[Kernel] Chain valid. Tip:" (:tip chain) "Depth:" (:depth chain))
        (log/error "[Kernel] Chain INVALID!"))
      chain)
    (catch Exception e
      (log/warn "[Kernel] Not reachable:" (.getMessage e))
      {:valid false :error (.getMessage e)})))

(defn demo! []
  (log/info "[Demo] Running system health check...")
  (let [chain (verify-kernel!)]
    (log/info "[Demo] Chain status:" chain))

  (log/info "[Demo] Testing planner...")
  (let [task   (planner/make-task "setup_and_ping" {:host "localhost"} false)
        p      (planner/plan task {})
        result (when p (planner/execute-plan! p {}))]
    (log/info "[Demo] Plan result:" result))

  (log/info "[Demo] Testing policy engine...")
  (log/info "[Demo] read_file allowed?" (policy/allowed? "read_file" {:path "/tmp/test"}))
  (log/info "[Demo] shell.exec allowed?" (policy/allowed? "shell.exec" {:cmd "ls"}))

  (log/info "[Demo] Complete."))

(defn -main [& args]
  (boot!)
  (let [cmd (first args)]
    (case cmd
      "demo"    (demo!)
      "verify"  (verify-kernel!)
      "agent"   (when-let [obj (second args)]
                  (run-agent! obj))
      "serve"   (do
                  ;; http-server (and the gRPC channel) are already running —
                  ;; started by mount/start in boot!. Block the main thread
                  ;; forever instead of falling through to shutdown!.
                  (log/info "[Serve] HTTP API listening on :" (or (System/getenv "CLJ_HTTP_PORT") "8090")
                            " — press Ctrl+C to stop.")
                  @(promise))
      ;; Default: demo
      (demo!))
    (when (not= cmd "serve")
      (Thread/sleep 500)
      (shutdown!))))
