(defproject omniharness-orchestrator "1.0.0"
  :description "OmniHarness Clojure Orchestrator — HTN planner, policy engine, patch manager"
  :dependencies [[org.clojure/clojure          "1.12.0"]
                 [io.grpc/grpc-netty-shaded     "1.68.1"]
                 [io.grpc/grpc-protobuf         "1.68.1"]
                 [io.grpc/grpc-stub             "1.68.1"]
                 [org.clojure/core.async        "1.6.681"]
                 [org.clojure/data.json         "2.5.0"]
                 [cheshire                      "5.13.0"]
                 ;; @javax.annotation.Generated used by the grpc-java stubs was
                 ;; removed from the JDK after Java 8 — supply it explicitly.
                 [javax.annotation/javax.annotation-api "1.3.2"]
                 [mount                         "0.1.17"]
                 [com.taoensso/timbre           "6.5.0"]
                 [http-kit                      "2.8.0"]
                 [ring/ring-core                "1.12.2"]
                 [ring/ring-json                "0.5.1"]
                 [compojure                     "1.7.1"]
                 [clj-http                      "3.13.0"]]
  :main omniharness.core
  :aot :all
  :source-paths ["src"]
  ;; gRPC/protobuf Java stubs generated from ../proto/omniharness.proto.
  ;; Regenerate with the tools/gen-proto.sh script (protoc + protoc-gen-grpc-java).
  :java-source-paths ["gen-java"]
  :javac-options ["-proc:none"]
  :profiles {:dev {:dependencies [[org.clojure/test.check "1.1.1"]]}})
