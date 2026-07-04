(ns omniharness.events
  "Event store client — appends and verifies the SHA-256 Merkle chain via gRPC."
  (:require [omniharness.client :refer [event-store-stub]]
            [cheshire.core :as json]
            [taoensso.timbre :as log])
  (:import [omniharness.v1
            AppendRequest VerifyRequest QueryRequest TipRequest]))

(defn append-event!
  "Append an event to the kernel event store. Returns event-hash."
  ([module-source event-type payload]
   (append-event! module-source event-type payload ""))
  ([module-source event-type payload session-id]
   (let [req (-> (AppendRequest/newBuilder)
                 (.setModuleSource (str module-source))
                 (.setEventType    (str event-type))
                 (.setPayloadJson  (if (string? payload) payload (json/generate-string payload)))
                 (.setSessionId    (str session-id))
                 .build)
         ^omniharness.v1.AppendResponse resp (.appendEvent @event-store-stub req)]
     (if (.getSuccess resp)
       (do (log/debug "Event appended:" event-type (.getEventHash resp))
           {:event-hash (.getEventHash resp)
            :event-id   (.getEventId resp)
            :success    true})
       (do (log/warn "Event append failed:" (.getError resp))
           {:success false :error (.getError resp)})))))

(defn verify-chain!
  "Verify the full Merkle chain. Returns {:valid bool :tip hash :depth n}."
  []
  (let [resp (.verifyChain @event-store-stub (VerifyRequest/getDefaultInstance))]
    {:valid (.getIsValid resp)
     :tip   (.getTipHash resp)
     :depth (.getDepth resp)}))

(defn get-tip!
  "Get current chain tip hash and depth."
  []
  (let [resp (.getTip @event-store-stub (TipRequest/getDefaultInstance))]
    {:tip   (.getTipHash resp)
     :depth (.getDepth resp)}))

(defn query-events!
  "Query events with optional filters. Returns seq of event maps."
  [{:keys [module event-type since-ts limit]
    :or   {module "" event-type "" since-ts 0 limit 100}}]
  (let [req (-> (QueryRequest/newBuilder)
                (.setModuleSource module)
                (.setEventType    event-type)
                (.setSinceTs      since-ts)
                (.setLimit        limit)
                .build)
        iter (.queryEvents @event-store-stub req)]
    (loop [acc []]
      (if (.hasNext iter)
        (let [ev (.next iter)]
          (recur (conj acc
                       {:id            (.getId ev)
                        :timestamp     (.getTimestampUtc ev)
                        :module-source (.getModuleSource ev)
                        :event-type    (.getEventType ev)
                        :payload       (try (json/parse-string (.getPayloadJson ev) true)
                                            (catch Exception _ (.getPayloadJson ev)))
                        :current-hash  (.getCurrentHash ev)})))
        acc))))
