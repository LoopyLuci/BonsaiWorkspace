#!/usr/bin/env bb
(ns bb-runner
  (:require [clojure.string :as str]))

(println "bb runner started — simple stdin health protocol")
(try
  (loop []
    (when-let [line (try (read-line) (catch Exception _ nil))]
      (let [cmd (str/trim line)]
        (if (= cmd "health")
          (println "{\"status\":\"ok\"}")
          (println "{\"error\":\"unknown\"}")))
      (recur)))
  (catch Exception e
    (println "bb-runner error:" e)))
