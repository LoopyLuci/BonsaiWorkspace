package omniharness.v1;

import static io.grpc.MethodDescriptor.generateFullMethodName;

/**
 */
@javax.annotation.Generated(
    value = "by gRPC proto compiler (version 1.68.1)",
    comments = "Source: omniharness.proto")
@io.grpc.stub.annotations.GrpcGenerated
public final class ToolServiceGrpc {

  private ToolServiceGrpc() {}

  public static final java.lang.String SERVICE_NAME = "omniharness.v1.ToolService";

  // Static method descriptors that strictly reflect the proto.
  private static volatile io.grpc.MethodDescriptor<omniharness.v1.ToolExecuteRequest,
      omniharness.v1.ToolExecuteResponse> getExecuteMethod;

  @io.grpc.stub.annotations.RpcMethod(
      fullMethodName = SERVICE_NAME + '/' + "Execute",
      requestType = omniharness.v1.ToolExecuteRequest.class,
      responseType = omniharness.v1.ToolExecuteResponse.class,
      methodType = io.grpc.MethodDescriptor.MethodType.UNARY)
  public static io.grpc.MethodDescriptor<omniharness.v1.ToolExecuteRequest,
      omniharness.v1.ToolExecuteResponse> getExecuteMethod() {
    io.grpc.MethodDescriptor<omniharness.v1.ToolExecuteRequest, omniharness.v1.ToolExecuteResponse> getExecuteMethod;
    if ((getExecuteMethod = ToolServiceGrpc.getExecuteMethod) == null) {
      synchronized (ToolServiceGrpc.class) {
        if ((getExecuteMethod = ToolServiceGrpc.getExecuteMethod) == null) {
          ToolServiceGrpc.getExecuteMethod = getExecuteMethod =
              io.grpc.MethodDescriptor.<omniharness.v1.ToolExecuteRequest, omniharness.v1.ToolExecuteResponse>newBuilder()
              .setType(io.grpc.MethodDescriptor.MethodType.UNARY)
              .setFullMethodName(generateFullMethodName(SERVICE_NAME, "Execute"))
              .setSampledToLocalTracing(true)
              .setRequestMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  omniharness.v1.ToolExecuteRequest.getDefaultInstance()))
              .setResponseMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  omniharness.v1.ToolExecuteResponse.getDefaultInstance()))
              .setSchemaDescriptor(new ToolServiceMethodDescriptorSupplier("Execute"))
              .build();
        }
      }
    }
    return getExecuteMethod;
  }

  private static volatile io.grpc.MethodDescriptor<omniharness.v1.ToolRegisterRequest,
      omniharness.v1.ToolRegisterResponse> getRegisterMethod;

  @io.grpc.stub.annotations.RpcMethod(
      fullMethodName = SERVICE_NAME + '/' + "Register",
      requestType = omniharness.v1.ToolRegisterRequest.class,
      responseType = omniharness.v1.ToolRegisterResponse.class,
      methodType = io.grpc.MethodDescriptor.MethodType.UNARY)
  public static io.grpc.MethodDescriptor<omniharness.v1.ToolRegisterRequest,
      omniharness.v1.ToolRegisterResponse> getRegisterMethod() {
    io.grpc.MethodDescriptor<omniharness.v1.ToolRegisterRequest, omniharness.v1.ToolRegisterResponse> getRegisterMethod;
    if ((getRegisterMethod = ToolServiceGrpc.getRegisterMethod) == null) {
      synchronized (ToolServiceGrpc.class) {
        if ((getRegisterMethod = ToolServiceGrpc.getRegisterMethod) == null) {
          ToolServiceGrpc.getRegisterMethod = getRegisterMethod =
              io.grpc.MethodDescriptor.<omniharness.v1.ToolRegisterRequest, omniharness.v1.ToolRegisterResponse>newBuilder()
              .setType(io.grpc.MethodDescriptor.MethodType.UNARY)
              .setFullMethodName(generateFullMethodName(SERVICE_NAME, "Register"))
              .setSampledToLocalTracing(true)
              .setRequestMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  omniharness.v1.ToolRegisterRequest.getDefaultInstance()))
              .setResponseMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  omniharness.v1.ToolRegisterResponse.getDefaultInstance()))
              .setSchemaDescriptor(new ToolServiceMethodDescriptorSupplier("Register"))
              .build();
        }
      }
    }
    return getRegisterMethod;
  }

  private static volatile io.grpc.MethodDescriptor<omniharness.v1.ToolListRequest,
      omniharness.v1.ToolListResponse> getListMethod;

  @io.grpc.stub.annotations.RpcMethod(
      fullMethodName = SERVICE_NAME + '/' + "List",
      requestType = omniharness.v1.ToolListRequest.class,
      responseType = omniharness.v1.ToolListResponse.class,
      methodType = io.grpc.MethodDescriptor.MethodType.UNARY)
  public static io.grpc.MethodDescriptor<omniharness.v1.ToolListRequest,
      omniharness.v1.ToolListResponse> getListMethod() {
    io.grpc.MethodDescriptor<omniharness.v1.ToolListRequest, omniharness.v1.ToolListResponse> getListMethod;
    if ((getListMethod = ToolServiceGrpc.getListMethod) == null) {
      synchronized (ToolServiceGrpc.class) {
        if ((getListMethod = ToolServiceGrpc.getListMethod) == null) {
          ToolServiceGrpc.getListMethod = getListMethod =
              io.grpc.MethodDescriptor.<omniharness.v1.ToolListRequest, omniharness.v1.ToolListResponse>newBuilder()
              .setType(io.grpc.MethodDescriptor.MethodType.UNARY)
              .setFullMethodName(generateFullMethodName(SERVICE_NAME, "List"))
              .setSampledToLocalTracing(true)
              .setRequestMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  omniharness.v1.ToolListRequest.getDefaultInstance()))
              .setResponseMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  omniharness.v1.ToolListResponse.getDefaultInstance()))
              .setSchemaDescriptor(new ToolServiceMethodDescriptorSupplier("List"))
              .build();
        }
      }
    }
    return getListMethod;
  }

  private static volatile io.grpc.MethodDescriptor<omniharness.v1.ToolUnregRequest,
      omniharness.v1.ToolUnregResponse> getUnregisterMethod;

  @io.grpc.stub.annotations.RpcMethod(
      fullMethodName = SERVICE_NAME + '/' + "Unregister",
      requestType = omniharness.v1.ToolUnregRequest.class,
      responseType = omniharness.v1.ToolUnregResponse.class,
      methodType = io.grpc.MethodDescriptor.MethodType.UNARY)
  public static io.grpc.MethodDescriptor<omniharness.v1.ToolUnregRequest,
      omniharness.v1.ToolUnregResponse> getUnregisterMethod() {
    io.grpc.MethodDescriptor<omniharness.v1.ToolUnregRequest, omniharness.v1.ToolUnregResponse> getUnregisterMethod;
    if ((getUnregisterMethod = ToolServiceGrpc.getUnregisterMethod) == null) {
      synchronized (ToolServiceGrpc.class) {
        if ((getUnregisterMethod = ToolServiceGrpc.getUnregisterMethod) == null) {
          ToolServiceGrpc.getUnregisterMethod = getUnregisterMethod =
              io.grpc.MethodDescriptor.<omniharness.v1.ToolUnregRequest, omniharness.v1.ToolUnregResponse>newBuilder()
              .setType(io.grpc.MethodDescriptor.MethodType.UNARY)
              .setFullMethodName(generateFullMethodName(SERVICE_NAME, "Unregister"))
              .setSampledToLocalTracing(true)
              .setRequestMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  omniharness.v1.ToolUnregRequest.getDefaultInstance()))
              .setResponseMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  omniharness.v1.ToolUnregResponse.getDefaultInstance()))
              .setSchemaDescriptor(new ToolServiceMethodDescriptorSupplier("Unregister"))
              .build();
        }
      }
    }
    return getUnregisterMethod;
  }

  /**
   * Creates a new async stub that supports all call types for the service
   */
  public static ToolServiceStub newStub(io.grpc.Channel channel) {
    io.grpc.stub.AbstractStub.StubFactory<ToolServiceStub> factory =
      new io.grpc.stub.AbstractStub.StubFactory<ToolServiceStub>() {
        @java.lang.Override
        public ToolServiceStub newStub(io.grpc.Channel channel, io.grpc.CallOptions callOptions) {
          return new ToolServiceStub(channel, callOptions);
        }
      };
    return ToolServiceStub.newStub(factory, channel);
  }

  /**
   * Creates a new blocking-style stub that supports unary and streaming output calls on the service
   */
  public static ToolServiceBlockingStub newBlockingStub(
      io.grpc.Channel channel) {
    io.grpc.stub.AbstractStub.StubFactory<ToolServiceBlockingStub> factory =
      new io.grpc.stub.AbstractStub.StubFactory<ToolServiceBlockingStub>() {
        @java.lang.Override
        public ToolServiceBlockingStub newStub(io.grpc.Channel channel, io.grpc.CallOptions callOptions) {
          return new ToolServiceBlockingStub(channel, callOptions);
        }
      };
    return ToolServiceBlockingStub.newStub(factory, channel);
  }

  /**
   * Creates a new ListenableFuture-style stub that supports unary calls on the service
   */
  public static ToolServiceFutureStub newFutureStub(
      io.grpc.Channel channel) {
    io.grpc.stub.AbstractStub.StubFactory<ToolServiceFutureStub> factory =
      new io.grpc.stub.AbstractStub.StubFactory<ToolServiceFutureStub>() {
        @java.lang.Override
        public ToolServiceFutureStub newStub(io.grpc.Channel channel, io.grpc.CallOptions callOptions) {
          return new ToolServiceFutureStub(channel, callOptions);
        }
      };
    return ToolServiceFutureStub.newStub(factory, channel);
  }

  /**
   */
  public interface AsyncService {

    /**
     */
    default void execute(omniharness.v1.ToolExecuteRequest request,
        io.grpc.stub.StreamObserver<omniharness.v1.ToolExecuteResponse> responseObserver) {
      io.grpc.stub.ServerCalls.asyncUnimplementedUnaryCall(getExecuteMethod(), responseObserver);
    }

    /**
     */
    default void register(omniharness.v1.ToolRegisterRequest request,
        io.grpc.stub.StreamObserver<omniharness.v1.ToolRegisterResponse> responseObserver) {
      io.grpc.stub.ServerCalls.asyncUnimplementedUnaryCall(getRegisterMethod(), responseObserver);
    }

    /**
     */
    default void list(omniharness.v1.ToolListRequest request,
        io.grpc.stub.StreamObserver<omniharness.v1.ToolListResponse> responseObserver) {
      io.grpc.stub.ServerCalls.asyncUnimplementedUnaryCall(getListMethod(), responseObserver);
    }

    /**
     */
    default void unregister(omniharness.v1.ToolUnregRequest request,
        io.grpc.stub.StreamObserver<omniharness.v1.ToolUnregResponse> responseObserver) {
      io.grpc.stub.ServerCalls.asyncUnimplementedUnaryCall(getUnregisterMethod(), responseObserver);
    }
  }

  /**
   * Base class for the server implementation of the service ToolService.
   */
  public static abstract class ToolServiceImplBase
      implements io.grpc.BindableService, AsyncService {

    @java.lang.Override public final io.grpc.ServerServiceDefinition bindService() {
      return ToolServiceGrpc.bindService(this);
    }
  }

  /**
   * A stub to allow clients to do asynchronous rpc calls to service ToolService.
   */
  public static final class ToolServiceStub
      extends io.grpc.stub.AbstractAsyncStub<ToolServiceStub> {
    private ToolServiceStub(
        io.grpc.Channel channel, io.grpc.CallOptions callOptions) {
      super(channel, callOptions);
    }

    @java.lang.Override
    protected ToolServiceStub build(
        io.grpc.Channel channel, io.grpc.CallOptions callOptions) {
      return new ToolServiceStub(channel, callOptions);
    }

    /**
     */
    public void execute(omniharness.v1.ToolExecuteRequest request,
        io.grpc.stub.StreamObserver<omniharness.v1.ToolExecuteResponse> responseObserver) {
      io.grpc.stub.ClientCalls.asyncUnaryCall(
          getChannel().newCall(getExecuteMethod(), getCallOptions()), request, responseObserver);
    }

    /**
     */
    public void register(omniharness.v1.ToolRegisterRequest request,
        io.grpc.stub.StreamObserver<omniharness.v1.ToolRegisterResponse> responseObserver) {
      io.grpc.stub.ClientCalls.asyncUnaryCall(
          getChannel().newCall(getRegisterMethod(), getCallOptions()), request, responseObserver);
    }

    /**
     */
    public void list(omniharness.v1.ToolListRequest request,
        io.grpc.stub.StreamObserver<omniharness.v1.ToolListResponse> responseObserver) {
      io.grpc.stub.ClientCalls.asyncUnaryCall(
          getChannel().newCall(getListMethod(), getCallOptions()), request, responseObserver);
    }

    /**
     */
    public void unregister(omniharness.v1.ToolUnregRequest request,
        io.grpc.stub.StreamObserver<omniharness.v1.ToolUnregResponse> responseObserver) {
      io.grpc.stub.ClientCalls.asyncUnaryCall(
          getChannel().newCall(getUnregisterMethod(), getCallOptions()), request, responseObserver);
    }
  }

  /**
   * A stub to allow clients to do synchronous rpc calls to service ToolService.
   */
  public static final class ToolServiceBlockingStub
      extends io.grpc.stub.AbstractBlockingStub<ToolServiceBlockingStub> {
    private ToolServiceBlockingStub(
        io.grpc.Channel channel, io.grpc.CallOptions callOptions) {
      super(channel, callOptions);
    }

    @java.lang.Override
    protected ToolServiceBlockingStub build(
        io.grpc.Channel channel, io.grpc.CallOptions callOptions) {
      return new ToolServiceBlockingStub(channel, callOptions);
    }

    /**
     */
    public omniharness.v1.ToolExecuteResponse execute(omniharness.v1.ToolExecuteRequest request) {
      return io.grpc.stub.ClientCalls.blockingUnaryCall(
          getChannel(), getExecuteMethod(), getCallOptions(), request);
    }

    /**
     */
    public omniharness.v1.ToolRegisterResponse register(omniharness.v1.ToolRegisterRequest request) {
      return io.grpc.stub.ClientCalls.blockingUnaryCall(
          getChannel(), getRegisterMethod(), getCallOptions(), request);
    }

    /**
     */
    public omniharness.v1.ToolListResponse list(omniharness.v1.ToolListRequest request) {
      return io.grpc.stub.ClientCalls.blockingUnaryCall(
          getChannel(), getListMethod(), getCallOptions(), request);
    }

    /**
     */
    public omniharness.v1.ToolUnregResponse unregister(omniharness.v1.ToolUnregRequest request) {
      return io.grpc.stub.ClientCalls.blockingUnaryCall(
          getChannel(), getUnregisterMethod(), getCallOptions(), request);
    }
  }

  /**
   * A stub to allow clients to do ListenableFuture-style rpc calls to service ToolService.
   */
  public static final class ToolServiceFutureStub
      extends io.grpc.stub.AbstractFutureStub<ToolServiceFutureStub> {
    private ToolServiceFutureStub(
        io.grpc.Channel channel, io.grpc.CallOptions callOptions) {
      super(channel, callOptions);
    }

    @java.lang.Override
    protected ToolServiceFutureStub build(
        io.grpc.Channel channel, io.grpc.CallOptions callOptions) {
      return new ToolServiceFutureStub(channel, callOptions);
    }

    /**
     */
    public com.google.common.util.concurrent.ListenableFuture<omniharness.v1.ToolExecuteResponse> execute(
        omniharness.v1.ToolExecuteRequest request) {
      return io.grpc.stub.ClientCalls.futureUnaryCall(
          getChannel().newCall(getExecuteMethod(), getCallOptions()), request);
    }

    /**
     */
    public com.google.common.util.concurrent.ListenableFuture<omniharness.v1.ToolRegisterResponse> register(
        omniharness.v1.ToolRegisterRequest request) {
      return io.grpc.stub.ClientCalls.futureUnaryCall(
          getChannel().newCall(getRegisterMethod(), getCallOptions()), request);
    }

    /**
     */
    public com.google.common.util.concurrent.ListenableFuture<omniharness.v1.ToolListResponse> list(
        omniharness.v1.ToolListRequest request) {
      return io.grpc.stub.ClientCalls.futureUnaryCall(
          getChannel().newCall(getListMethod(), getCallOptions()), request);
    }

    /**
     */
    public com.google.common.util.concurrent.ListenableFuture<omniharness.v1.ToolUnregResponse> unregister(
        omniharness.v1.ToolUnregRequest request) {
      return io.grpc.stub.ClientCalls.futureUnaryCall(
          getChannel().newCall(getUnregisterMethod(), getCallOptions()), request);
    }
  }

  private static final int METHODID_EXECUTE = 0;
  private static final int METHODID_REGISTER = 1;
  private static final int METHODID_LIST = 2;
  private static final int METHODID_UNREGISTER = 3;

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
        case METHODID_EXECUTE:
          serviceImpl.execute((omniharness.v1.ToolExecuteRequest) request,
              (io.grpc.stub.StreamObserver<omniharness.v1.ToolExecuteResponse>) responseObserver);
          break;
        case METHODID_REGISTER:
          serviceImpl.register((omniharness.v1.ToolRegisterRequest) request,
              (io.grpc.stub.StreamObserver<omniharness.v1.ToolRegisterResponse>) responseObserver);
          break;
        case METHODID_LIST:
          serviceImpl.list((omniharness.v1.ToolListRequest) request,
              (io.grpc.stub.StreamObserver<omniharness.v1.ToolListResponse>) responseObserver);
          break;
        case METHODID_UNREGISTER:
          serviceImpl.unregister((omniharness.v1.ToolUnregRequest) request,
              (io.grpc.stub.StreamObserver<omniharness.v1.ToolUnregResponse>) responseObserver);
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
          getExecuteMethod(),
          io.grpc.stub.ServerCalls.asyncUnaryCall(
            new MethodHandlers<
              omniharness.v1.ToolExecuteRequest,
              omniharness.v1.ToolExecuteResponse>(
                service, METHODID_EXECUTE)))
        .addMethod(
          getRegisterMethod(),
          io.grpc.stub.ServerCalls.asyncUnaryCall(
            new MethodHandlers<
              omniharness.v1.ToolRegisterRequest,
              omniharness.v1.ToolRegisterResponse>(
                service, METHODID_REGISTER)))
        .addMethod(
          getListMethod(),
          io.grpc.stub.ServerCalls.asyncUnaryCall(
            new MethodHandlers<
              omniharness.v1.ToolListRequest,
              omniharness.v1.ToolListResponse>(
                service, METHODID_LIST)))
        .addMethod(
          getUnregisterMethod(),
          io.grpc.stub.ServerCalls.asyncUnaryCall(
            new MethodHandlers<
              omniharness.v1.ToolUnregRequest,
              omniharness.v1.ToolUnregResponse>(
                service, METHODID_UNREGISTER)))
        .build();
  }

  private static abstract class ToolServiceBaseDescriptorSupplier
      implements io.grpc.protobuf.ProtoFileDescriptorSupplier, io.grpc.protobuf.ProtoServiceDescriptorSupplier {
    ToolServiceBaseDescriptorSupplier() {}

    @java.lang.Override
    public com.google.protobuf.Descriptors.FileDescriptor getFileDescriptor() {
      return omniharness.v1.Omniharness.getDescriptor();
    }

    @java.lang.Override
    public com.google.protobuf.Descriptors.ServiceDescriptor getServiceDescriptor() {
      return getFileDescriptor().findServiceByName("ToolService");
    }
  }

  private static final class ToolServiceFileDescriptorSupplier
      extends ToolServiceBaseDescriptorSupplier {
    ToolServiceFileDescriptorSupplier() {}
  }

  private static final class ToolServiceMethodDescriptorSupplier
      extends ToolServiceBaseDescriptorSupplier
      implements io.grpc.protobuf.ProtoMethodDescriptorSupplier {
    private final java.lang.String methodName;

    ToolServiceMethodDescriptorSupplier(java.lang.String methodName) {
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
      synchronized (ToolServiceGrpc.class) {
        result = serviceDescriptor;
        if (result == null) {
          serviceDescriptor = result = io.grpc.ServiceDescriptor.newBuilder(SERVICE_NAME)
              .setSchemaDescriptor(new ToolServiceFileDescriptorSupplier())
              .addMethod(getExecuteMethod())
              .addMethod(getRegisterMethod())
              .addMethod(getListMethod())
              .addMethod(getUnregisterMethod())
              .build();
        }
      }
    }
    return result;
  }
}
