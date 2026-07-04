package omniharness.v1;

import static io.grpc.MethodDescriptor.generateFullMethodName;

/**
 */
@javax.annotation.Generated(
    value = "by gRPC proto compiler (version 1.68.1)",
    comments = "Source: omniharness.proto")
@io.grpc.stub.annotations.GrpcGenerated
public final class HarnessServiceGrpc {

  private HarnessServiceGrpc() {}

  public static final java.lang.String SERVICE_NAME = "omniharness.v1.HarnessService";

  // Static method descriptors that strictly reflect the proto.
  private static volatile io.grpc.MethodDescriptor<omniharness.v1.StatusRequest,
      omniharness.v1.StatusResponse> getStatusMethod;

  @io.grpc.stub.annotations.RpcMethod(
      fullMethodName = SERVICE_NAME + '/' + "Status",
      requestType = omniharness.v1.StatusRequest.class,
      responseType = omniharness.v1.StatusResponse.class,
      methodType = io.grpc.MethodDescriptor.MethodType.UNARY)
  public static io.grpc.MethodDescriptor<omniharness.v1.StatusRequest,
      omniharness.v1.StatusResponse> getStatusMethod() {
    io.grpc.MethodDescriptor<omniharness.v1.StatusRequest, omniharness.v1.StatusResponse> getStatusMethod;
    if ((getStatusMethod = HarnessServiceGrpc.getStatusMethod) == null) {
      synchronized (HarnessServiceGrpc.class) {
        if ((getStatusMethod = HarnessServiceGrpc.getStatusMethod) == null) {
          HarnessServiceGrpc.getStatusMethod = getStatusMethod =
              io.grpc.MethodDescriptor.<omniharness.v1.StatusRequest, omniharness.v1.StatusResponse>newBuilder()
              .setType(io.grpc.MethodDescriptor.MethodType.UNARY)
              .setFullMethodName(generateFullMethodName(SERVICE_NAME, "Status"))
              .setSampledToLocalTracing(true)
              .setRequestMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  omniharness.v1.StatusRequest.getDefaultInstance()))
              .setResponseMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  omniharness.v1.StatusResponse.getDefaultInstance()))
              .setSchemaDescriptor(new HarnessServiceMethodDescriptorSupplier("Status"))
              .build();
        }
      }
    }
    return getStatusMethod;
  }

  private static volatile io.grpc.MethodDescriptor<omniharness.v1.ConfigRequest,
      omniharness.v1.ConfigResponse> getConfigMethod;

  @io.grpc.stub.annotations.RpcMethod(
      fullMethodName = SERVICE_NAME + '/' + "Config",
      requestType = omniharness.v1.ConfigRequest.class,
      responseType = omniharness.v1.ConfigResponse.class,
      methodType = io.grpc.MethodDescriptor.MethodType.UNARY)
  public static io.grpc.MethodDescriptor<omniharness.v1.ConfigRequest,
      omniharness.v1.ConfigResponse> getConfigMethod() {
    io.grpc.MethodDescriptor<omniharness.v1.ConfigRequest, omniharness.v1.ConfigResponse> getConfigMethod;
    if ((getConfigMethod = HarnessServiceGrpc.getConfigMethod) == null) {
      synchronized (HarnessServiceGrpc.class) {
        if ((getConfigMethod = HarnessServiceGrpc.getConfigMethod) == null) {
          HarnessServiceGrpc.getConfigMethod = getConfigMethod =
              io.grpc.MethodDescriptor.<omniharness.v1.ConfigRequest, omniharness.v1.ConfigResponse>newBuilder()
              .setType(io.grpc.MethodDescriptor.MethodType.UNARY)
              .setFullMethodName(generateFullMethodName(SERVICE_NAME, "Config"))
              .setSampledToLocalTracing(true)
              .setRequestMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  omniharness.v1.ConfigRequest.getDefaultInstance()))
              .setResponseMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  omniharness.v1.ConfigResponse.getDefaultInstance()))
              .setSchemaDescriptor(new HarnessServiceMethodDescriptorSupplier("Config"))
              .build();
        }
      }
    }
    return getConfigMethod;
  }

  private static volatile io.grpc.MethodDescriptor<omniharness.v1.ReloadRequest,
      omniharness.v1.ReloadResponse> getReloadMethod;

  @io.grpc.stub.annotations.RpcMethod(
      fullMethodName = SERVICE_NAME + '/' + "Reload",
      requestType = omniharness.v1.ReloadRequest.class,
      responseType = omniharness.v1.ReloadResponse.class,
      methodType = io.grpc.MethodDescriptor.MethodType.UNARY)
  public static io.grpc.MethodDescriptor<omniharness.v1.ReloadRequest,
      omniharness.v1.ReloadResponse> getReloadMethod() {
    io.grpc.MethodDescriptor<omniharness.v1.ReloadRequest, omniharness.v1.ReloadResponse> getReloadMethod;
    if ((getReloadMethod = HarnessServiceGrpc.getReloadMethod) == null) {
      synchronized (HarnessServiceGrpc.class) {
        if ((getReloadMethod = HarnessServiceGrpc.getReloadMethod) == null) {
          HarnessServiceGrpc.getReloadMethod = getReloadMethod =
              io.grpc.MethodDescriptor.<omniharness.v1.ReloadRequest, omniharness.v1.ReloadResponse>newBuilder()
              .setType(io.grpc.MethodDescriptor.MethodType.UNARY)
              .setFullMethodName(generateFullMethodName(SERVICE_NAME, "Reload"))
              .setSampledToLocalTracing(true)
              .setRequestMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  omniharness.v1.ReloadRequest.getDefaultInstance()))
              .setResponseMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  omniharness.v1.ReloadResponse.getDefaultInstance()))
              .setSchemaDescriptor(new HarnessServiceMethodDescriptorSupplier("Reload"))
              .build();
        }
      }
    }
    return getReloadMethod;
  }

  private static volatile io.grpc.MethodDescriptor<omniharness.v1.MetricsRequest,
      omniharness.v1.MetricsResponse> getMetricsMethod;

  @io.grpc.stub.annotations.RpcMethod(
      fullMethodName = SERVICE_NAME + '/' + "Metrics",
      requestType = omniharness.v1.MetricsRequest.class,
      responseType = omniharness.v1.MetricsResponse.class,
      methodType = io.grpc.MethodDescriptor.MethodType.UNARY)
  public static io.grpc.MethodDescriptor<omniharness.v1.MetricsRequest,
      omniharness.v1.MetricsResponse> getMetricsMethod() {
    io.grpc.MethodDescriptor<omniharness.v1.MetricsRequest, omniharness.v1.MetricsResponse> getMetricsMethod;
    if ((getMetricsMethod = HarnessServiceGrpc.getMetricsMethod) == null) {
      synchronized (HarnessServiceGrpc.class) {
        if ((getMetricsMethod = HarnessServiceGrpc.getMetricsMethod) == null) {
          HarnessServiceGrpc.getMetricsMethod = getMetricsMethod =
              io.grpc.MethodDescriptor.<omniharness.v1.MetricsRequest, omniharness.v1.MetricsResponse>newBuilder()
              .setType(io.grpc.MethodDescriptor.MethodType.UNARY)
              .setFullMethodName(generateFullMethodName(SERVICE_NAME, "Metrics"))
              .setSampledToLocalTracing(true)
              .setRequestMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  omniharness.v1.MetricsRequest.getDefaultInstance()))
              .setResponseMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  omniharness.v1.MetricsResponse.getDefaultInstance()))
              .setSchemaDescriptor(new HarnessServiceMethodDescriptorSupplier("Metrics"))
              .build();
        }
      }
    }
    return getMetricsMethod;
  }

  /**
   * Creates a new async stub that supports all call types for the service
   */
  public static HarnessServiceStub newStub(io.grpc.Channel channel) {
    io.grpc.stub.AbstractStub.StubFactory<HarnessServiceStub> factory =
      new io.grpc.stub.AbstractStub.StubFactory<HarnessServiceStub>() {
        @java.lang.Override
        public HarnessServiceStub newStub(io.grpc.Channel channel, io.grpc.CallOptions callOptions) {
          return new HarnessServiceStub(channel, callOptions);
        }
      };
    return HarnessServiceStub.newStub(factory, channel);
  }

  /**
   * Creates a new blocking-style stub that supports unary and streaming output calls on the service
   */
  public static HarnessServiceBlockingStub newBlockingStub(
      io.grpc.Channel channel) {
    io.grpc.stub.AbstractStub.StubFactory<HarnessServiceBlockingStub> factory =
      new io.grpc.stub.AbstractStub.StubFactory<HarnessServiceBlockingStub>() {
        @java.lang.Override
        public HarnessServiceBlockingStub newStub(io.grpc.Channel channel, io.grpc.CallOptions callOptions) {
          return new HarnessServiceBlockingStub(channel, callOptions);
        }
      };
    return HarnessServiceBlockingStub.newStub(factory, channel);
  }

  /**
   * Creates a new ListenableFuture-style stub that supports unary calls on the service
   */
  public static HarnessServiceFutureStub newFutureStub(
      io.grpc.Channel channel) {
    io.grpc.stub.AbstractStub.StubFactory<HarnessServiceFutureStub> factory =
      new io.grpc.stub.AbstractStub.StubFactory<HarnessServiceFutureStub>() {
        @java.lang.Override
        public HarnessServiceFutureStub newStub(io.grpc.Channel channel, io.grpc.CallOptions callOptions) {
          return new HarnessServiceFutureStub(channel, callOptions);
        }
      };
    return HarnessServiceFutureStub.newStub(factory, channel);
  }

  /**
   */
  public interface AsyncService {

    /**
     */
    default void status(omniharness.v1.StatusRequest request,
        io.grpc.stub.StreamObserver<omniharness.v1.StatusResponse> responseObserver) {
      io.grpc.stub.ServerCalls.asyncUnimplementedUnaryCall(getStatusMethod(), responseObserver);
    }

    /**
     */
    default void config(omniharness.v1.ConfigRequest request,
        io.grpc.stub.StreamObserver<omniharness.v1.ConfigResponse> responseObserver) {
      io.grpc.stub.ServerCalls.asyncUnimplementedUnaryCall(getConfigMethod(), responseObserver);
    }

    /**
     */
    default void reload(omniharness.v1.ReloadRequest request,
        io.grpc.stub.StreamObserver<omniharness.v1.ReloadResponse> responseObserver) {
      io.grpc.stub.ServerCalls.asyncUnimplementedUnaryCall(getReloadMethod(), responseObserver);
    }

    /**
     */
    default void metrics(omniharness.v1.MetricsRequest request,
        io.grpc.stub.StreamObserver<omniharness.v1.MetricsResponse> responseObserver) {
      io.grpc.stub.ServerCalls.asyncUnimplementedUnaryCall(getMetricsMethod(), responseObserver);
    }
  }

  /**
   * Base class for the server implementation of the service HarnessService.
   */
  public static abstract class HarnessServiceImplBase
      implements io.grpc.BindableService, AsyncService {

    @java.lang.Override public final io.grpc.ServerServiceDefinition bindService() {
      return HarnessServiceGrpc.bindService(this);
    }
  }

  /**
   * A stub to allow clients to do asynchronous rpc calls to service HarnessService.
   */
  public static final class HarnessServiceStub
      extends io.grpc.stub.AbstractAsyncStub<HarnessServiceStub> {
    private HarnessServiceStub(
        io.grpc.Channel channel, io.grpc.CallOptions callOptions) {
      super(channel, callOptions);
    }

    @java.lang.Override
    protected HarnessServiceStub build(
        io.grpc.Channel channel, io.grpc.CallOptions callOptions) {
      return new HarnessServiceStub(channel, callOptions);
    }

    /**
     */
    public void status(omniharness.v1.StatusRequest request,
        io.grpc.stub.StreamObserver<omniharness.v1.StatusResponse> responseObserver) {
      io.grpc.stub.ClientCalls.asyncUnaryCall(
          getChannel().newCall(getStatusMethod(), getCallOptions()), request, responseObserver);
    }

    /**
     */
    public void config(omniharness.v1.ConfigRequest request,
        io.grpc.stub.StreamObserver<omniharness.v1.ConfigResponse> responseObserver) {
      io.grpc.stub.ClientCalls.asyncUnaryCall(
          getChannel().newCall(getConfigMethod(), getCallOptions()), request, responseObserver);
    }

    /**
     */
    public void reload(omniharness.v1.ReloadRequest request,
        io.grpc.stub.StreamObserver<omniharness.v1.ReloadResponse> responseObserver) {
      io.grpc.stub.ClientCalls.asyncUnaryCall(
          getChannel().newCall(getReloadMethod(), getCallOptions()), request, responseObserver);
    }

    /**
     */
    public void metrics(omniharness.v1.MetricsRequest request,
        io.grpc.stub.StreamObserver<omniharness.v1.MetricsResponse> responseObserver) {
      io.grpc.stub.ClientCalls.asyncUnaryCall(
          getChannel().newCall(getMetricsMethod(), getCallOptions()), request, responseObserver);
    }
  }

  /**
   * A stub to allow clients to do synchronous rpc calls to service HarnessService.
   */
  public static final class HarnessServiceBlockingStub
      extends io.grpc.stub.AbstractBlockingStub<HarnessServiceBlockingStub> {
    private HarnessServiceBlockingStub(
        io.grpc.Channel channel, io.grpc.CallOptions callOptions) {
      super(channel, callOptions);
    }

    @java.lang.Override
    protected HarnessServiceBlockingStub build(
        io.grpc.Channel channel, io.grpc.CallOptions callOptions) {
      return new HarnessServiceBlockingStub(channel, callOptions);
    }

    /**
     */
    public omniharness.v1.StatusResponse status(omniharness.v1.StatusRequest request) {
      return io.grpc.stub.ClientCalls.blockingUnaryCall(
          getChannel(), getStatusMethod(), getCallOptions(), request);
    }

    /**
     */
    public omniharness.v1.ConfigResponse config(omniharness.v1.ConfigRequest request) {
      return io.grpc.stub.ClientCalls.blockingUnaryCall(
          getChannel(), getConfigMethod(), getCallOptions(), request);
    }

    /**
     */
    public omniharness.v1.ReloadResponse reload(omniharness.v1.ReloadRequest request) {
      return io.grpc.stub.ClientCalls.blockingUnaryCall(
          getChannel(), getReloadMethod(), getCallOptions(), request);
    }

    /**
     */
    public omniharness.v1.MetricsResponse metrics(omniharness.v1.MetricsRequest request) {
      return io.grpc.stub.ClientCalls.blockingUnaryCall(
          getChannel(), getMetricsMethod(), getCallOptions(), request);
    }
  }

  /**
   * A stub to allow clients to do ListenableFuture-style rpc calls to service HarnessService.
   */
  public static final class HarnessServiceFutureStub
      extends io.grpc.stub.AbstractFutureStub<HarnessServiceFutureStub> {
    private HarnessServiceFutureStub(
        io.grpc.Channel channel, io.grpc.CallOptions callOptions) {
      super(channel, callOptions);
    }

    @java.lang.Override
    protected HarnessServiceFutureStub build(
        io.grpc.Channel channel, io.grpc.CallOptions callOptions) {
      return new HarnessServiceFutureStub(channel, callOptions);
    }

    /**
     */
    public com.google.common.util.concurrent.ListenableFuture<omniharness.v1.StatusResponse> status(
        omniharness.v1.StatusRequest request) {
      return io.grpc.stub.ClientCalls.futureUnaryCall(
          getChannel().newCall(getStatusMethod(), getCallOptions()), request);
    }

    /**
     */
    public com.google.common.util.concurrent.ListenableFuture<omniharness.v1.ConfigResponse> config(
        omniharness.v1.ConfigRequest request) {
      return io.grpc.stub.ClientCalls.futureUnaryCall(
          getChannel().newCall(getConfigMethod(), getCallOptions()), request);
    }

    /**
     */
    public com.google.common.util.concurrent.ListenableFuture<omniharness.v1.ReloadResponse> reload(
        omniharness.v1.ReloadRequest request) {
      return io.grpc.stub.ClientCalls.futureUnaryCall(
          getChannel().newCall(getReloadMethod(), getCallOptions()), request);
    }

    /**
     */
    public com.google.common.util.concurrent.ListenableFuture<omniharness.v1.MetricsResponse> metrics(
        omniharness.v1.MetricsRequest request) {
      return io.grpc.stub.ClientCalls.futureUnaryCall(
          getChannel().newCall(getMetricsMethod(), getCallOptions()), request);
    }
  }

  private static final int METHODID_STATUS = 0;
  private static final int METHODID_CONFIG = 1;
  private static final int METHODID_RELOAD = 2;
  private static final int METHODID_METRICS = 3;

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
        case METHODID_STATUS:
          serviceImpl.status((omniharness.v1.StatusRequest) request,
              (io.grpc.stub.StreamObserver<omniharness.v1.StatusResponse>) responseObserver);
          break;
        case METHODID_CONFIG:
          serviceImpl.config((omniharness.v1.ConfigRequest) request,
              (io.grpc.stub.StreamObserver<omniharness.v1.ConfigResponse>) responseObserver);
          break;
        case METHODID_RELOAD:
          serviceImpl.reload((omniharness.v1.ReloadRequest) request,
              (io.grpc.stub.StreamObserver<omniharness.v1.ReloadResponse>) responseObserver);
          break;
        case METHODID_METRICS:
          serviceImpl.metrics((omniharness.v1.MetricsRequest) request,
              (io.grpc.stub.StreamObserver<omniharness.v1.MetricsResponse>) responseObserver);
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
          getStatusMethod(),
          io.grpc.stub.ServerCalls.asyncUnaryCall(
            new MethodHandlers<
              omniharness.v1.StatusRequest,
              omniharness.v1.StatusResponse>(
                service, METHODID_STATUS)))
        .addMethod(
          getConfigMethod(),
          io.grpc.stub.ServerCalls.asyncUnaryCall(
            new MethodHandlers<
              omniharness.v1.ConfigRequest,
              omniharness.v1.ConfigResponse>(
                service, METHODID_CONFIG)))
        .addMethod(
          getReloadMethod(),
          io.grpc.stub.ServerCalls.asyncUnaryCall(
            new MethodHandlers<
              omniharness.v1.ReloadRequest,
              omniharness.v1.ReloadResponse>(
                service, METHODID_RELOAD)))
        .addMethod(
          getMetricsMethod(),
          io.grpc.stub.ServerCalls.asyncUnaryCall(
            new MethodHandlers<
              omniharness.v1.MetricsRequest,
              omniharness.v1.MetricsResponse>(
                service, METHODID_METRICS)))
        .build();
  }

  private static abstract class HarnessServiceBaseDescriptorSupplier
      implements io.grpc.protobuf.ProtoFileDescriptorSupplier, io.grpc.protobuf.ProtoServiceDescriptorSupplier {
    HarnessServiceBaseDescriptorSupplier() {}

    @java.lang.Override
    public com.google.protobuf.Descriptors.FileDescriptor getFileDescriptor() {
      return omniharness.v1.Omniharness.getDescriptor();
    }

    @java.lang.Override
    public com.google.protobuf.Descriptors.ServiceDescriptor getServiceDescriptor() {
      return getFileDescriptor().findServiceByName("HarnessService");
    }
  }

  private static final class HarnessServiceFileDescriptorSupplier
      extends HarnessServiceBaseDescriptorSupplier {
    HarnessServiceFileDescriptorSupplier() {}
  }

  private static final class HarnessServiceMethodDescriptorSupplier
      extends HarnessServiceBaseDescriptorSupplier
      implements io.grpc.protobuf.ProtoMethodDescriptorSupplier {
    private final java.lang.String methodName;

    HarnessServiceMethodDescriptorSupplier(java.lang.String methodName) {
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
      synchronized (HarnessServiceGrpc.class) {
        result = serviceDescriptor;
        if (result == null) {
          serviceDescriptor = result = io.grpc.ServiceDescriptor.newBuilder(SERVICE_NAME)
              .setSchemaDescriptor(new HarnessServiceFileDescriptorSupplier())
              .addMethod(getStatusMethod())
              .addMethod(getConfigMethod())
              .addMethod(getReloadMethod())
              .addMethod(getMetricsMethod())
              .build();
        }
      }
    }
    return result;
  }
}
