#!/usr/bin/env bb
(ns launch-preflight
  (:require [babashka.fs :as fs]
            [babashka.process :as p]
            [babashka.cli :as cli]
            [cheshire.core :as json]))

(def defaults
  {:host "127.0.0.1"
   :report "tool_test/launcher/latest-bb.json"})

(defn now-iso []
  (.toString (java.time.Instant/now)))

(defn run* [& args]
  (let [{:keys [exit out err]} (apply p/shell {:out :string :err :string :continue true} args)]
    {:exit exit :out (or out "") :err (or err "")}))

(defn version! [cmd & args]
  (let [{:keys [exit out err]} (apply run* cmd args)
        txt (clojure.string/trim (str out err))]
    (if (zero? exit)
      txt
  (throw (ex-info (str cmd " check failed") {:cmd cmd :args args :out out :err err :exit exit})))))

(defn read-api-port []
  (try
    (let [appdata (System/getenv "APPDATA")
          cfg (when (seq appdata)
                (str appdata "\\com.bonsai.workspace\\bonsai-config.json"))]
      (if (and cfg (fs/exists? cfg))
        (let [m (json/parse-string (slurp cfg) true)
              p (long (or (:api_port m) 11369))]
          (if (<= 1 p 65535) p 11369))
        11369))
    (catch Exception _ 11369)))

(defn api-healthy? [host port]
  (try
    (let [url (format "http://%s:%d/health" host port)
          client (java.net.http.HttpClient/newHttpClient)
          req (-> (java.net.http.HttpRequest/newBuilder (java.net.URI/create url))
                  (.timeout (java.time.Duration/ofSeconds 2))
                  (.GET)
                  (.build))
          resp (.send client req (java.net.http.HttpResponse$BodyHandlers/ofString))]
      (= 200 (.statusCode resp)))
    (catch Exception _ false)))

(defn port-listening? [port]
  (let [{:keys [exit out]} (run* "netstat" "-ano")
        needle (str ":" port)]
    (and (zero? exit)
         (some #(and (clojure.string/includes? % needle)
                     (re-find #"(?i)LISTEN" %))
               (clojure.string/split-lines out)))))

(defn write-report! [path report]
  (let [abs (str (fs/absolutize path))]
    (fs/create-dirs (fs/parent abs))
    (spit abs (json/generate-string report {:pretty true}))
    abs))

(defn -main [& argv]
  (let [{:keys [report host]} (merge defaults (cli/parse-args argv {:coerce {:report :string :host :string}}))
        started (now-iso)
        port (read-api-port)]
    (try
      (let [versions {:node  (version! "node" "--version")
                      :npm   (version! "npm" "--version")
                      :cargo (version! "cargo" "--version")
                      :tauri (version! "cargo" "tauri" "--version")}
            listening (port-listening? port)
            healthy (api-healthy? host port)
            report-map {:schema_version 1
                        :ts started
                        :finished_at (now-iso)
                        :ok true
                        :mode "preflight-bb"
                        :host host
                        :api_port port
                        :details {:versions versions
                                  :api_port_listening listening
                                  :api_healthy healthy}}]
        (println "[bb-preflight] node:" (:node versions))
        (println "[bb-preflight] npm:" (:npm versions))
        (println "[bb-preflight] cargo:" (:cargo versions))
        (println "[bb-preflight] tauri:" (:tauri versions))
        (when listening
          (println (format "[bb-preflight] port %d is listening (%shealthy)" port (if healthy "" "not "))))
        (let [out (write-report! report report-map)]
          (println "[bb-preflight] report written:" out))
        (System/exit 0))
      (catch Exception e
        (let [data (or (ex-data e) {})
              report-map {:schema_version 1
                          :ts started
                          :finished_at (now-iso)
                          :ok false
                          :mode "preflight-bb"
                          :host host
                          :api_port port
                          :error {:message (.getMessage e)
                                  :data data}}]
          (println "[bb-preflight] error:" (.getMessage e))
          (let [out (write-report! report report-map)]
            (println "[bb-preflight] report written:" out))
          (System/exit 1))))))

(apply -main *command-line-args*)
