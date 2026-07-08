(ns omniharness.http-server
  "Real HTTP surface for the Clojure orchestrator's HTN planner and policy
   engine — makes clj-orchestrator a genuine, callable service instead of a
   demo-only CLI nothing else invokes. Consumed by
   orchestrator/omniharness/clj_client.py with graceful degradation when this
   process isn't running (mirrors the kernel gRPC bridge's pattern).
   Started as part of normal `lein run serve` boot (see core.clj)."
  (:require [org.httpkit.server :as http-kit]
            [compojure.core :refer [defroutes GET POST]]
            [compojure.route :as route]
            [ring.middleware.json :refer [wrap-json-body wrap-json-response]]
            [ring.util.response :refer [response]]
            [mount.core :refer [defstate]]
            [taoensso.timbre :as log]
            [omniharness.planner :as planner]
            [omniharness.policy :as policy]
            [omniharness.events :as events]))

(defn- safe
  "Runs f, returning {:ok true :result ...} or {:ok false :error ...} — every
   route uses this so a kernel-unreachable exception becomes a normal JSON
   error response instead of a 500 with no explanation."
  [f]
  (try
    {:ok true :result (f)}
    (catch Exception e
      {:ok false :error (.getMessage e)})))

(defroutes app-routes
  (GET "/health" [] (response {:status "ok" :service "clj-orchestrator"}))

  (GET "/kernel/verify" []
    (response (safe events/verify-chain!)))

  (POST "/plan" {body :body}
    (let [{:keys [task_name params]} body
          state (or params {})
          task  (planner/make-task task_name state false)]
      (response (safe (fn []
                         (let [p (planner/plan task state)]
                           {:plan (or p []) :found (some? p)}))))))

  (POST "/plan/execute" {body :body}
    (let [{:keys [task_name params]} body
          state (or params {})
          task  (planner/make-task task_name state false)]
      (response (safe (fn []
                         (let [p (planner/plan task state)]
                           (if p
                             (planner/execute-plan! p state)
                             {:error "No valid plan found for task" :task task_name})))))))

  (POST "/policy/check" {body :body}
    (let [{:keys [action args]} body]
      (response (safe (fn []
                         (let [decision (policy/evaluate action (or args {}))]
                           {:action action :decision (name decision) :allowed (= decision :allow)}))))))

  (route/not-found (response {:error "not found"})))

(def handler
  (-> app-routes
      (wrap-json-body {:keywords? true})
      wrap-json-response))

(def ^:private http-port
  (Integer/parseInt (or (System/getenv "CLJ_HTTP_PORT") "8090")))

(defstate http-server
  :start (do (log/info "[http] Starting clj-orchestrator HTTP API on :" http-port)
             (http-kit/run-server handler {:port http-port}))
  :stop  (when http-server (http-server :timeout 100)))
