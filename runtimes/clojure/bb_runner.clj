#!/usr/bin/env bb
(ns bb-runner
  (:require [clojure.java.io :as io]
            [clojure.string :as str]))

(defn path-separator []
  (if (.startsWith (System/getProperty "os.name" "") "Windows") ";" ":"))

(defn parse-allowed-paths []
  (let [args (vec *command-line-args*)
        idx (.indexOf args "--allowed-paths")
        from-arg (when (and (>= idx 0) (< (inc idx) (count args)))
                   (nth args (inc idx)))
        from-env (System/getenv "BONSAI_ALLOWED_PATHS")
        raw (or from-arg from-env "")]
    (->> (str/split raw (re-pattern (java.util.regex.Pattern/quote (path-separator))))
         (map str/trim)
         (remove str/blank?)
         (map #(try (.getCanonicalPath (io/file %)) (catch Exception _ %)))
         vec)))

(defn allowed-path? [allowed p]
  (if (str/blank? p)
    false
    (let [canon (try (.getCanonicalPath (io/file p))
                     (catch Exception _ p))]
      (boolean
       (some (fn [root]
               (or (= canon root)
                   (.startsWith canon (str root java.io.File/separator))))
             allowed)))))

(def allowed-paths (parse-allowed-paths))

(println "bb runner started — simple stdin health protocol")
(try
  (loop []
    (when-let [line (try (read-line) (catch Exception _ nil))]
      (let [cmd (str/trim line)]
        (cond
          (= cmd "health")
          (println "{\"status\":\"ok\"}")

          (str/starts-with? cmd "check-path ")
          (let [target (subs cmd (count "check-path "))]
            (if (allowed-path? allowed-paths target)
              (println "{\"status\":\"ok\",\"path\":\"allowed\"}")
              (println "{\"error\":\"path_not_allowed\"}")))

          :else
          (println "{\"error\":\"unknown\"}")))
      (recur)))
  (catch Exception e
    (println "bb-runner error:" e)))
