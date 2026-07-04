(ns omniharness.client
  "gRPC channel and stub management for all OmniHarness services."
  (:require [mount.core :refer [defstate]]
            [taoensso.timbre :as log])
  (:import [io.grpc ManagedChannelBuilder ManagedChannel]
           [omniharness.v1
            EventStoreServiceGrpc
            ModelServiceGrpc
            MemoryServiceGrpc
            ToolServiceGrpc
            SessionServiceGrpc
            HarnessServiceGrpc]))

(def ^:private grpc-host (or (System/getenv "OMNIHARNESS_GRPC_HOST") "localhost"))
(def ^:private grpc-port (Integer/parseInt (or (System/getenv "OMNIHARNESS_GRPC_PORT") "50051")))

(defn create-channel ^ManagedChannel []
  (-> (ManagedChannelBuilder/forAddress grpc-host grpc-port)
      .usePlaintext
      .build))

(defstate channel
  :start (do (log/info "[gRPC] Connecting to" (str grpc-host ":" grpc-port))
             (create-channel))
  :stop  (do (log/info "[gRPC] Shutting down channel.")
             (.shutdown ^ManagedChannel channel)))

(defstate event-store-stub
  :start (EventStoreServiceGrpc/newBlockingStub channel)
  :stop  nil)

(defstate model-stub
  :start (ModelServiceGrpc/newBlockingStub channel)
  :stop  nil)

(defstate memory-stub
  :start (MemoryServiceGrpc/newBlockingStub channel)
  :stop  nil)

(defstate tool-stub
  :start (ToolServiceGrpc/newBlockingStub channel)
  :stop  nil)

(defstate session-stub
  :start (SessionServiceGrpc/newBlockingStub channel)
  :stop  nil)

(defstate harness-stub
  :start (HarnessServiceGrpc/newBlockingStub channel)
  :stop  nil)
