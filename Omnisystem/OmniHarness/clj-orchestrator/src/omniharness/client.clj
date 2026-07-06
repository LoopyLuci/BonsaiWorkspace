(ns omniharness.client
  "gRPC channel and stub management for all OmniHarness services."
  (:require [mount.core :refer [defstate]]
            [taoensso.timbre :as log])
  (:import [io.grpc ManagedChannelBuilder ManagedChannel Metadata Metadata$Key ClientInterceptor]
           [io.grpc.stub MetadataUtils]
           [omniharness.v1
            EventStoreServiceGrpc
            ModelServiceGrpc
            MemoryServiceGrpc
            ToolServiceGrpc
            SessionServiceGrpc
            HarnessServiceGrpc]))

(def ^:private grpc-host (or (System/getenv "OMNIHARNESS_GRPC_HOST") "localhost"))
(def ^:private grpc-port (Integer/parseInt (or (System/getenv "OMNIHARNESS_GRPC_PORT") "50051")))
(def ^:private admin-key (System/getenv "OMNIHARNESS_ADMIN_KEY"))

(defn create-channel ^ManagedChannel []
  (-> (ManagedChannelBuilder/forAddress grpc-host grpc-port)
      .usePlaintext
      .build))

(defn- attach-auth
  "Wraps a blocking stub so every call carries x-omniharness-key metadata
   when OMNIHARNESS_ADMIN_KEY is set — a no-op otherwise (the kernel's
   default, OMNIHARNESS_REQUIRE_AUTH-unset state doesn't check this header
   at all; see kernel/src/grpc_server.rs's AuthInterceptor)."
  [stub]
  (if admin-key
    (let [md (doto (Metadata.)
               (.put (Metadata$Key/of "x-omniharness-key" Metadata/ASCII_STRING_MARSHALLER) admin-key))
          interceptor (MetadataUtils/newAttachHeadersInterceptor md)]
      (.withInterceptors stub (into-array ClientInterceptor [interceptor])))
    stub))

(defstate channel
  :start (do (log/info "[gRPC] Connecting to" (str grpc-host ":" grpc-port))
             (create-channel))
  :stop  (do (log/info "[gRPC] Shutting down channel.")
             (.shutdown ^ManagedChannel channel)))

(defstate event-store-stub
  :start (attach-auth (EventStoreServiceGrpc/newBlockingStub channel))
  :stop  nil)

(defstate model-stub
  :start (attach-auth (ModelServiceGrpc/newBlockingStub channel))
  :stop  nil)

(defstate memory-stub
  :start (attach-auth (MemoryServiceGrpc/newBlockingStub channel))
  :stop  nil)

(defstate tool-stub
  :start (attach-auth (ToolServiceGrpc/newBlockingStub channel))
  :stop  nil)

(defstate session-stub
  :start (attach-auth (SessionServiceGrpc/newBlockingStub channel))
  :stop  nil)

(defstate harness-stub
  :start (attach-auth (HarnessServiceGrpc/newBlockingStub channel))
  :stop  nil)
