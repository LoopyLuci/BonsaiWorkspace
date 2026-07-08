package omniharness.v1;

import static io.grpc.MethodDescriptor.generateFullMethodName;

/**
 */
@javax.annotation.Generated(
    value = "by gRPC proto compiler (version 1.68.1)",
    comments = "Source: omniharness.proto")
@io.grpc.stub.annotations.GrpcGenerated
public final class EventStoreServiceGrpc {

  private EventStoreServiceGrpc() {}

  public static final java.lang.String SERVICE_NAME = "omniharness.v1.EventStoreService";

  // Static method descriptors that strictly reflect the proto.
  private static volatile io.grpc.MethodDescriptor<omniharness.v1.AppendRequest,
      omniharness.v1.AppendResponse> getAppendEventMethod;

  @io.grpc.stub.annotations.RpcMethod(
      fullMethodName = SERVICE_NAME + '/' + "AppendEvent",
      requestType = omniharness.v1.AppendRequest.class,
      responseType = omniharness.v1.AppendResponse.class,
      methodType = io.grpc.MethodDescriptor.MethodType.UNARY)
  public static io.grpc.MethodDescriptor<omniharness.v1.AppendRequest,
      omniharness.v1.AppendResponse> getAppendEventMethod() {
    io.grpc.MethodDescriptor<omniharness.v1.AppendRequest, omniharness.v1.AppendResponse> getAppendEventMethod;
    if ((getAppendEventMethod = EventStoreServiceGrpc.getAppendEventMethod) == null) {
      synchronized (EventStoreServiceGrpc.class) {
        if ((getAppendEventMethod = EventStoreServiceGrpc.getAppendEventMethod) == null) {
          EventStoreServiceGrpc.getAppendEventMethod = getAppendEventMethod =
              io.grpc.MethodDescriptor.<omniharness.v1.AppendRequest, omniharness.v1.AppendResponse>newBuilder()
              .setType(io.grpc.MethodDescriptor.MethodType.UNARY)
              .setFullMethodName(generateFullMethodName(SERVICE_NAME, "AppendEvent"))
              .setSampledToLocalTracing(true)
              .setRequestMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  omniharness.v1.AppendRequest.getDefaultInstance()))
              .setResponseMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  omniharness.v1.AppendResponse.getDefaultInstance()))
              .setSchemaDescriptor(new EventStoreServiceMethodDescriptorSupplier("AppendEvent"))
              .build();
        }
      }
    }
    return getAppendEventMethod;
  }

  private static volatile io.grpc.MethodDescriptor<omniharness.v1.VerifyRequest,
      omniharness.v1.VerifyResponse> getVerifyChainMethod;

  @io.grpc.stub.annotations.RpcMethod(
      fullMethodName = SERVICE_NAME + '/' + "VerifyChain",
      requestType = omniharness.v1.VerifyRequest.class,
      responseType = omniharness.v1.VerifyResponse.class,
      methodType = io.grpc.MethodDescriptor.MethodType.UNARY)
  public static io.grpc.MethodDescriptor<omniharness.v1.VerifyRequest,
      omniharness.v1.VerifyResponse> getVerifyChainMethod() {
    io.grpc.MethodDescriptor<omniharness.v1.VerifyRequest, omniharness.v1.VerifyResponse> getVerifyChainMethod;
    if ((getVerifyChainMethod = EventStoreServiceGrpc.getVerifyChainMethod) == null) {
      synchronized (EventStoreServiceGrpc.class) {
        if ((getVerifyChainMethod = EventStoreServiceGrpc.getVerifyChainMethod) == null) {
          EventStoreServiceGrpc.getVerifyChainMethod = getVerifyChainMethod =
              io.grpc.MethodDescriptor.<omniharness.v1.VerifyRequest, omniharness.v1.VerifyResponse>newBuilder()
              .setType(io.grpc.MethodDescriptor.MethodType.UNARY)
              .setFullMethodName(generateFullMethodName(SERVICE_NAME, "VerifyChain"))
              .setSampledToLocalTracing(true)
              .setRequestMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  omniharness.v1.VerifyRequest.getDefaultInstance()))
              .setResponseMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  omniharness.v1.VerifyResponse.getDefaultInstance()))
              .setSchemaDescriptor(new EventStoreServiceMethodDescriptorSupplier("VerifyChain"))
              .build();
        }
      }
    }
    return getVerifyChainMethod;
  }

  private static volatile io.grpc.MethodDescriptor<omniharness.v1.QueryRequest,
      omniharness.v1.SystemEvent> getQueryEventsMethod;

  @io.grpc.stub.annotations.RpcMethod(
      fullMethodName = SERVICE_NAME + '/' + "QueryEvents",
      requestType = omniharness.v1.QueryRequest.class,
      responseType = omniharness.v1.SystemEvent.class,
      methodType = io.grpc.MethodDescriptor.MethodType.SERVER_STREAMING)
  public static io.grpc.MethodDescriptor<omniharness.v1.QueryRequest,
      omniharness.v1.SystemEvent> getQueryEventsMethod() {
    io.grpc.MethodDescriptor<omniharness.v1.QueryRequest, omniharness.v1.SystemEvent> getQueryEventsMethod;
    if ((getQueryEventsMethod = EventStoreServiceGrpc.getQueryEventsMethod) == null) {
      synchronized (EventStoreServiceGrpc.class) {
        if ((getQueryEventsMethod = EventStoreServiceGrpc.getQueryEventsMethod) == null) {
          EventStoreServiceGrpc.getQueryEventsMethod = getQueryEventsMethod =
              io.grpc.MethodDescriptor.<omniharness.v1.QueryRequest, omniharness.v1.SystemEvent>newBuilder()
              .setType(io.grpc.MethodDescriptor.MethodType.SERVER_STREAMING)
              .setFullMethodName(generateFullMethodName(SERVICE_NAME, "QueryEvents"))
              .setSampledToLocalTracing(true)
              .setRequestMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  omniharness.v1.QueryRequest.getDefaultInstance()))
              .setResponseMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  omniharness.v1.SystemEvent.getDefaultInstance()))
              .setSchemaDescriptor(new EventStoreServiceMethodDescriptorSupplier("QueryEvents"))
              .build();
        }
      }
    }
    return getQueryEventsMethod;
  }

  private static volatile io.grpc.MethodDescriptor<omniharness.v1.TipRequest,
      omniharness.v1.TipResponse> getGetTipMethod;

  @io.grpc.stub.annotations.RpcMethod(
      fullMethodName = SERVICE_NAME + '/' + "GetTip",
      requestType = omniharness.v1.TipRequest.class,
      responseType = omniharness.v1.TipResponse.class,
      methodType = io.grpc.MethodDescriptor.MethodType.UNARY)
  public static io.grpc.MethodDescriptor<omniharness.v1.TipRequest,
      omniharness.v1.TipResponse> getGetTipMethod() {
    io.grpc.MethodDescriptor<omniharness.v1.TipRequest, omniharness.v1.TipResponse> getGetTipMethod;
    if ((getGetTipMethod = EventStoreServiceGrpc.getGetTipMethod) == null) {
      synchronized (EventStoreServiceGrpc.class) {
        if ((getGetTipMethod = EventStoreServiceGrpc.getGetTipMethod) == null) {
          EventStoreServiceGrpc.getGetTipMethod = getGetTipMethod =
              io.grpc.MethodDescriptor.<omniharness.v1.TipRequest, omniharness.v1.TipResponse>newBuilder()
              .setType(io.grpc.MethodDescriptor.MethodType.UNARY)
              .setFullMethodName(generateFullMethodName(SERVICE_NAME, "GetTip"))
              .setSampledToLocalTracing(true)
              .setRequestMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  omniharness.v1.TipRequest.getDefaultInstance()))
              .setResponseMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  omniharness.v1.TipResponse.getDefaultInstance()))
              .setSchemaDescriptor(new EventStoreServiceMethodDescriptorSupplier("GetTip"))
              .build();
        }
      }
    }
    return getGetTipMethod;
  }

  /**
   * Creates a new async stub that supports all call types for the service
   */
  public static EventStoreServiceStub newStub(io.grpc.Channel channel) {
    io.grpc.stub.AbstractStub.StubFactory<EventStoreServiceStub> factory =
      new io.grpc.stub.AbstractStub.StubFactory<EventStoreServiceStub>() {
        @java.lang.Override
        public EventStoreServiceStub newStub(io.grpc.Channel channel, io.grpc.CallOptions callOptions) {
          return new EventStoreServiceStub(channel, callOptions);
        }
      };
    return EventStoreServiceStub.newStub(factory, channel);
  }

  /**
   * Creates a new blocking-style stub that supports unary and streaming output calls on the service
   */
  public static EventStoreServiceBlockingStub newBlockingStub(
      io.grpc.Channel channel) {
    io.grpc.stub.AbstractStub.StubFactory<EventStoreServiceBlockingStub> factory =
      new io.grpc.stub.AbstractStub.StubFactory<EventStoreServiceBlockingStub>() {
        @java.lang.Override
        public EventStoreServiceBlockingStub newStub(io.grpc.Channel channel, io.grpc.CallOptions callOptions) {
          return new EventStoreServiceBlockingStub(channel, callOptions);
        }
      };
    return EventStoreServiceBlockingStub.newStub(factory, channel);
  }

  /**
   * Creates a new ListenableFuture-style stub that supports unary calls on the service
   */
  public static EventStoreServiceFutureStub newFutureStub(
      io.grpc.Channel channel) {
    io.grpc.stub.AbstractStub.StubFactory<EventStoreServiceFutureStub> factory =
      new io.grpc.stub.AbstractStub.StubFactory<EventStoreServiceFutureStub>() {
        @java.lang.Override
        public EventStoreServiceFutureStub newStub(io.grpc.Channel channel, io.grpc.CallOptions callOptions) {
          return new EventStoreServiceFutureStub(channel, callOptions);
        }
      };
    return EventStoreServiceFutureStub.newStub(factory, channel);
  }

  /**
   */
  public interface AsyncService {

    /**
     */
    default void appendEvent(omniharness.v1.AppendRequest request,
        io.grpc.stub.StreamObserver<omniharness.v1.AppendResponse> responseObserver) {
      io.grpc.stub.ServerCalls.asyncUnimplementedUnaryCall(getAppendEventMethod(), responseObserver);
    }

    /**
     */
    default void verifyChain(omniharness.v1.VerifyRequest request,
        io.grpc.stub.StreamObserver<omniharness.v1.VerifyResponse> responseObserver) {
      io.grpc.stub.ServerCalls.asyncUnimplementedUnaryCall(getVerifyChainMethod(), responseObserver);
    }

    /**
     */
    default void queryEvents(omniharness.v1.QueryRequest request,
        io.grpc.stub.StreamObserver<omniharness.v1.SystemEvent> responseObserver) {
      io.grpc.stub.ServerCalls.asyncUnimplementedUnaryCall(getQueryEventsMethod(), responseObserver);
    }

    /**
     */
    default void getTip(omniharness.v1.TipRequest request,
        io.grpc.stub.StreamObserver<omniharness.v1.TipResponse> responseObserver) {
      io.grpc.stub.ServerCalls.asyncUnimplementedUnaryCall(getGetTipMethod(), responseObserver);
    }
  }

  /**
   * Base class for the server implementation of the service EventStoreService.
   */
  public static abstract class EventStoreServiceImplBase
      implements io.grpc.BindableService, AsyncService {

    @java.lang.Override public final io.grpc.ServerServiceDefinition bindService() {
      return EventStoreServiceGrpc.bindService(this);
    }
  }

  /**
   * A stub to allow clients to do asynchronous rpc calls to service EventStoreService.
   */
  public static final class EventStoreServiceStub
      extends io.grpc.stub.AbstractAsyncStub<EventStoreServiceStub> {
    private EventStoreServiceStub(
        io.grpc.Channel channel, io.grpc.CallOptions callOptions) {
      super(channel, callOptions);
    }

    @java.lang.Override
    protected EventStoreServiceStub build(
        io.grpc.Channel channel, io.grpc.CallOptions callOptions) {
      return new EventStoreServiceStub(channel, callOptions);
    }

    /**
     */
    public void appendEvent(omniharness.v1.AppendRequest request,
        io.grpc.stub.StreamObserver<omniharness.v1.AppendResponse> responseObserver) {
      io.grpc.stub.ClientCalls.asyncUnaryCall(
          getChannel().newCall(getAppendEventMethod(), getCallOptions()), request, responseObserver);
    }

    /**
     */
    public void verifyChain(omniharness.v1.VerifyRequest request,
        io.grpc.stub.StreamObserver<omniharness.v1.VerifyResponse> responseObserver) {
      io.grpc.stub.ClientCalls.asyncUnaryCall(
          getChannel().newCall(getVerifyChainMethod(), getCallOptions()), request, responseObserver);
    }

    /**
     */
    public void queryEvents(omniharness.v1.QueryRequest request,
        io.grpc.stub.StreamObserver<omniharness.v1.SystemEvent> responseObserver) {
      io.grpc.stub.ClientCalls.asyncServerStreamingCall(
          getChannel().newCall(getQueryEventsMethod(), getCallOptions()), request, responseObserver);
    }

    /**
     */
    public void getTip(omniharness.v1.TipRequest request,
        io.grpc.stub.StreamObserver<omniharness.v1.TipResponse> responseObserver) {
      io.grpc.stub.ClientCalls.asyncUnaryCall(
          getChannel().newCall(getGetTipMethod(), getCallOptions()), request, responseObserver);
    }
  }

  /**
   * A stub to allow clients to do synchronous rpc calls to service EventStoreService.
   */
  public static final class EventStoreServiceBlockingStub
      extends io.grpc.stub.AbstractBlockingStub<EventStoreServiceBlockingStub> {
    private EventStoreServiceBlockingStub(
        io.grpc.Channel channel, io.grpc.CallOptions callOptions) {
      super(channel, callOptions);
    }

    @java.lang.Override
    protected EventStoreServiceBlockingStub build(
        io.grpc.Channel channel, io.grpc.CallOptions callOptions) {
      return new EventStoreServiceBlockingStub(channel, callOptions);
    }

    /**
     */
    public omniharness.v1.AppendResponse appendEvent(omniharness.v1.AppendRequest request) {
      return io.grpc.stub.ClientCalls.blockingUnaryCall(
          getChannel(), getAppendEventMethod(), getCallOptions(), request);
    }

    /**
     */
    public omniharness.v1.VerifyResponse verifyChain(omniharness.v1.VerifyRequest request) {
      return io.grpc.stub.ClientCalls.blockingUnaryCall(
          getChannel(), getVerifyChainMethod(), getCallOptions(), request);
    }

    /**
     */
    public java.util.Iterator<omniharness.v1.SystemEvent> queryEvents(
        omniharness.v1.QueryRequest request) {
      return io.grpc.stub.ClientCalls.blockingServerStreamingCall(
          getChannel(), getQueryEventsMethod(), getCallOptions(), request);
    }

    /**
     */
    public omniharness.v1.TipResponse getTip(omniharness.v1.TipRequest request) {
      return io.grpc.stub.ClientCalls.blockingUnaryCall(
          getChannel(), getGetTipMethod(), getCallOptions(), request);
    }
  }

  /**
   * A stub to allow clients to do ListenableFuture-style rpc calls to service EventStoreService.
   */
  public static final class EventStoreServiceFutureStub
      extends io.grpc.stub.AbstractFutureStub<EventStoreServiceFutureStub> {
    private EventStoreServiceFutureStub(
        io.grpc.Channel channel, io.grpc.CallOptions callOptions) {
      super(channel, callOptions);
    }

    @java.lang.Override
    protected EventStoreServiceFutureStub build(
        io.grpc.Channel channel, io.grpc.CallOptions callOptions) {
      return new EventStoreServiceFutureStub(channel, callOptions);
    }

    /**
     */
    public com.google.common.util.concurrent.ListenableFuture<omniharness.v1.AppendResponse> appendEvent(
        omniharness.v1.AppendRequest request) {
      return io.grpc.stub.ClientCalls.futureUnaryCall(
          getChannel().newCall(getAppendEventMethod(), getCallOptions()), request);
    }

    /**
     */
    public com.google.common.util.concurrent.ListenableFuture<omniharness.v1.VerifyResponse> verifyChain(
        omniharness.v1.VerifyRequest request) {
      return io.grpc.stub.ClientCalls.futureUnaryCall(
          getChannel().newCall(getVerifyChainMethod(), getCallOptions()), request);
    }

    /**
     */
    public com.google.common.util.concurrent.ListenableFuture<omniharness.v1.TipResponse> getTip(
        omniharness.v1.TipRequest request) {
      return io.grpc.stub.ClientCalls.futureUnaryCall(
          getChannel().newCall(getGetTipMethod(), getCallOptions()), request);
    }
  }

  private static final int METHODID_APPEND_EVENT = 0;
  private static final int METHODID_VERIFY_CHAIN = 1;
  private static final int METHODID_QUERY_EVENTS = 2;
  private static final int METHODID_GET_TIP = 3;

  private static final class MethodHandlers<Req, Resp> implements
      io.grpc.stub.ServerCalls.UnaryMethod<Req, Resp>,
      io.grpc.stub.ServerCalls.ServerStreamingMethod<Req, Resp>,
      io.grpc.stub.ServerCalls.ClientStreamingMethod<Req, Resp>,
      io.grpc.stub.ServerCalls.BidiStreamingMethod<Req, Resp> {
    private final AsyncService serviceImpl;
    private final int methodId;

    MethodHandlers(AsyncService serviceImpl, int methodId) {
      this.serviceImpl = serviceImpl;
      this.methodId = methodId;
    }

    @java.lang.Override
    @java.lang.SuppressWarnings("unchecked")
    public void invoke(Req request, io.grpc.stub.StreamObserver<Resp> responseObserver) {
      switch (methodId) {
        case METHODID_APPEND_EVENT:
          serviceImpl.appendEvent((omniharness.v1.AppendRequest) request,
              (io.grpc.stub.StreamObserver<omniharness.v1.AppendResponse>) responseObserver);
          break;
        case METHODID_VERIFY_CHAIN:
          serviceImpl.verifyChain((omniharness.v1.VerifyRequest) request,
              (io.grpc.stub.StreamObserver<omniharness.v1.VerifyResponse>) responseObserver);
          break;
        case METHODID_QUERY_EVENTS:
          serviceImpl.queryEvents((omniharness.v1.QueryRequest) request,
              (io.grpc.stub.StreamObserver<omniharness.v1.SystemEvent>) responseObserver);
          break;
        case METHODID_GET_TIP:
          serviceImpl.getTip((omniharness.v1.TipRequest) request,
              (io.grpc.stub.StreamObserver<omniharness.v1.TipResponse>) responseObserver);
          break;
        default:
          throw new AssertionError();
      }
    }

    @java.lang.Override
    @java.lang.SuppressWarnings("unchecked")
    public io.grpc.stub.StreamObserver<Req> invoke(
        io.grpc.stub.StreamObserver<Resp> responseObserver) {
      switch (methodId) {
        default:
          throw new AssertionError();
      }
    }
  }

  public static final io.grpc.ServerServiceDefinition bindService(AsyncService service) {
    return io.grpc.ServerServiceDefinition.builder(getServiceDescriptor())
        .addMethod(
          getAppendEventMethod(),
          io.grpc.stub.ServerCalls.asyncUnaryCall(
            new MethodHandlers<
              omniharness.v1.AppendRequest,
              omniharness.v1.AppendResponse>(
                service, METHODID_APPEND_EVENT)))
        .addMethod(
          getVerifyChainMethod(),
          io.grpc.stub.ServerCalls.asyncUnaryCall(
            new MethodHandlers<
              omniharness.v1.VerifyRequest,
              omniharness.v1.VerifyResponse>(
                service, METHODID_VERIFY_CHAIN)))
        .addMethod(
          getQueryEventsMethod(),
          io.grpc.stub.ServerCalls.asyncServerStreamingCall(
            new MethodHandlers<
              omniharness.v1.QueryRequest,
              omniharness.v1.SystemEvent>(
                service, METHODID_QUERY_EVENTS)))
        .addMethod(
          getGetTipMethod(),
          io.grpc.stub.ServerCalls.asyncUnaryCall(
            new MethodHandlers<
              omniharness.v1.TipRequest,
              omniharness.v1.TipResponse>(
                service, METHODID_GET_TIP)))
        .build();
  }

  private static abstract class EventStoreServiceBaseDescriptorSupplier
      implements io.grpc.protobuf.ProtoFileDescriptorSupplier, io.grpc.protobuf.ProtoServiceDescriptorSupplier {
    EventStoreServiceBaseDescriptorSupplier() {}

    @java.lang.Override
    public com.google.protobuf.Descriptors.FileDescriptor getFileDescriptor() {
      return omniharness.v1.Omniharness.getDescriptor();
    }

    @java.lang.Override
    public com.google.protobuf.Descriptors.ServiceDescriptor getServiceDescriptor() {
      return getFileDescriptor().findServiceByName("EventStoreService");
    }
  }

  private static final class EventStoreServiceFileDescriptorSupplier
      extends EventStoreServiceBaseDescriptorSupplier {
    EventStoreServiceFileDescriptorSupplier() {}
  }

  private static final class EventStoreServiceMethodDescriptorSupplier
      extends EventStoreServiceBaseDescriptorSupplier
      implements io.grpc.protobuf.ProtoMethodDescriptorSupplier {
    private final java.lang.String methodName;

    EventStoreServiceMethodDescriptorSupplier(java.lang.String methodName) {
      this.methodName = methodName;
    }

    @java.lang.Override
    public com.google.protobuf.Descriptors.MethodDescriptor getMethodDescriptor() {
      return getServiceDescriptor().findMethodByName(methodName);
    }
  }

  private static volatile io.grpc.ServiceDescriptor serviceDescriptor;

  public static io.grpc.ServiceDescriptor getServiceDescriptor() {
    io.grpc.ServiceDescriptor result = serviceDescriptor;
    if (result == null) {
      synchronized (EventStoreServiceGrpc.class) {
        result = serviceDescriptor;
        if (result == null) {
          serviceDescriptor = result = io.grpc.ServiceDescriptor.newBuilder(SERVICE_NAME)
              .setSchemaDescriptor(new EventStoreServiceFileDescriptorSupplier())
              .addMethod(getAppendEventMethod())
              .addMethod(getVerifyChainMethod())
              .addMethod(getQueryEventsMethod())
              .addMethod(getGetTipMethod())
              .build();
        }
      }
    }
    return result;
  }
}
