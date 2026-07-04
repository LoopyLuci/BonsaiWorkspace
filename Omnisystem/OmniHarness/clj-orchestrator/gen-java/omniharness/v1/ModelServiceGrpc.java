package omniharness.v1;

import static io.grpc.MethodDescriptor.generateFullMethodName;

/**
 */
@javax.annotation.Generated(
    value = "by gRPC proto compiler (version 1.68.1)",
    comments = "Source: omniharness.proto")
@io.grpc.stub.annotations.GrpcGenerated
public final class ModelServiceGrpc {

  private ModelServiceGrpc() {}

  public static final java.lang.String SERVICE_NAME = "omniharness.v1.ModelService";

  // Static method descriptors that strictly reflect the proto.
  private static volatile io.grpc.MethodDescriptor<omniharness.v1.ChatRequest,
      omniharness.v1.ChatResponse> getChatMethod;

  @io.grpc.stub.annotations.RpcMethod(
      fullMethodName = SERVICE_NAME + '/' + "Chat",
      requestType = omniharness.v1.ChatRequest.class,
      responseType = omniharness.v1.ChatResponse.class,
      methodType = io.grpc.MethodDescriptor.MethodType.UNARY)
  public static io.grpc.MethodDescriptor<omniharness.v1.ChatRequest,
      omniharness.v1.ChatResponse> getChatMethod() {
    io.grpc.MethodDescriptor<omniharness.v1.ChatRequest, omniharness.v1.ChatResponse> getChatMethod;
    if ((getChatMethod = ModelServiceGrpc.getChatMethod) == null) {
      synchronized (ModelServiceGrpc.class) {
        if ((getChatMethod = ModelServiceGrpc.getChatMethod) == null) {
          ModelServiceGrpc.getChatMethod = getChatMethod =
              io.grpc.MethodDescriptor.<omniharness.v1.ChatRequest, omniharness.v1.ChatResponse>newBuilder()
              .setType(io.grpc.MethodDescriptor.MethodType.UNARY)
              .setFullMethodName(generateFullMethodName(SERVICE_NAME, "Chat"))
              .setSampledToLocalTracing(true)
              .setRequestMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  omniharness.v1.ChatRequest.getDefaultInstance()))
              .setResponseMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  omniharness.v1.ChatResponse.getDefaultInstance()))
              .setSchemaDescriptor(new ModelServiceMethodDescriptorSupplier("Chat"))
              .build();
        }
      }
    }
    return getChatMethod;
  }

  private static volatile io.grpc.MethodDescriptor<omniharness.v1.ChatRequest,
      omniharness.v1.ChatChunk> getChatStreamMethod;

  @io.grpc.stub.annotations.RpcMethod(
      fullMethodName = SERVICE_NAME + '/' + "ChatStream",
      requestType = omniharness.v1.ChatRequest.class,
      responseType = omniharness.v1.ChatChunk.class,
      methodType = io.grpc.MethodDescriptor.MethodType.SERVER_STREAMING)
  public static io.grpc.MethodDescriptor<omniharness.v1.ChatRequest,
      omniharness.v1.ChatChunk> getChatStreamMethod() {
    io.grpc.MethodDescriptor<omniharness.v1.ChatRequest, omniharness.v1.ChatChunk> getChatStreamMethod;
    if ((getChatStreamMethod = ModelServiceGrpc.getChatStreamMethod) == null) {
      synchronized (ModelServiceGrpc.class) {
        if ((getChatStreamMethod = ModelServiceGrpc.getChatStreamMethod) == null) {
          ModelServiceGrpc.getChatStreamMethod = getChatStreamMethod =
              io.grpc.MethodDescriptor.<omniharness.v1.ChatRequest, omniharness.v1.ChatChunk>newBuilder()
              .setType(io.grpc.MethodDescriptor.MethodType.SERVER_STREAMING)
              .setFullMethodName(generateFullMethodName(SERVICE_NAME, "ChatStream"))
              .setSampledToLocalTracing(true)
              .setRequestMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  omniharness.v1.ChatRequest.getDefaultInstance()))
              .setResponseMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  omniharness.v1.ChatChunk.getDefaultInstance()))
              .setSchemaDescriptor(new ModelServiceMethodDescriptorSupplier("ChatStream"))
              .build();
        }
      }
    }
    return getChatStreamMethod;
  }

  private static volatile io.grpc.MethodDescriptor<omniharness.v1.ListModelsRequest,
      omniharness.v1.ListModelsResponse> getListModelsMethod;

  @io.grpc.stub.annotations.RpcMethod(
      fullMethodName = SERVICE_NAME + '/' + "ListModels",
      requestType = omniharness.v1.ListModelsRequest.class,
      responseType = omniharness.v1.ListModelsResponse.class,
      methodType = io.grpc.MethodDescriptor.MethodType.UNARY)
  public static io.grpc.MethodDescriptor<omniharness.v1.ListModelsRequest,
      omniharness.v1.ListModelsResponse> getListModelsMethod() {
    io.grpc.MethodDescriptor<omniharness.v1.ListModelsRequest, omniharness.v1.ListModelsResponse> getListModelsMethod;
    if ((getListModelsMethod = ModelServiceGrpc.getListModelsMethod) == null) {
      synchronized (ModelServiceGrpc.class) {
        if ((getListModelsMethod = ModelServiceGrpc.getListModelsMethod) == null) {
          ModelServiceGrpc.getListModelsMethod = getListModelsMethod =
              io.grpc.MethodDescriptor.<omniharness.v1.ListModelsRequest, omniharness.v1.ListModelsResponse>newBuilder()
              .setType(io.grpc.MethodDescriptor.MethodType.UNARY)
              .setFullMethodName(generateFullMethodName(SERVICE_NAME, "ListModels"))
              .setSampledToLocalTracing(true)
              .setRequestMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  omniharness.v1.ListModelsRequest.getDefaultInstance()))
              .setResponseMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  omniharness.v1.ListModelsResponse.getDefaultInstance()))
              .setSchemaDescriptor(new ModelServiceMethodDescriptorSupplier("ListModels"))
              .build();
        }
      }
    }
    return getListModelsMethod;
  }

  private static volatile io.grpc.MethodDescriptor<omniharness.v1.ModelHealthRequest,
      omniharness.v1.ModelHealthResponse> getHealthCheckMethod;

  @io.grpc.stub.annotations.RpcMethod(
      fullMethodName = SERVICE_NAME + '/' + "HealthCheck",
      requestType = omniharness.v1.ModelHealthRequest.class,
      responseType = omniharness.v1.ModelHealthResponse.class,
      methodType = io.grpc.MethodDescriptor.MethodType.UNARY)
  public static io.grpc.MethodDescriptor<omniharness.v1.ModelHealthRequest,
      omniharness.v1.ModelHealthResponse> getHealthCheckMethod() {
    io.grpc.MethodDescriptor<omniharness.v1.ModelHealthRequest, omniharness.v1.ModelHealthResponse> getHealthCheckMethod;
    if ((getHealthCheckMethod = ModelServiceGrpc.getHealthCheckMethod) == null) {
      synchronized (ModelServiceGrpc.class) {
        if ((getHealthCheckMethod = ModelServiceGrpc.getHealthCheckMethod) == null) {
          ModelServiceGrpc.getHealthCheckMethod = getHealthCheckMethod =
              io.grpc.MethodDescriptor.<omniharness.v1.ModelHealthRequest, omniharness.v1.ModelHealthResponse>newBuilder()
              .setType(io.grpc.MethodDescriptor.MethodType.UNARY)
              .setFullMethodName(generateFullMethodName(SERVICE_NAME, "HealthCheck"))
              .setSampledToLocalTracing(true)
              .setRequestMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  omniharness.v1.ModelHealthRequest.getDefaultInstance()))
              .setResponseMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  omniharness.v1.ModelHealthResponse.getDefaultInstance()))
              .setSchemaDescriptor(new ModelServiceMethodDescriptorSupplier("HealthCheck"))
              .build();
        }
      }
    }
    return getHealthCheckMethod;
  }

  private static volatile io.grpc.MethodDescriptor<omniharness.v1.RegisterModelReq,
      omniharness.v1.RegisterModelResp> getRegisterModelMethod;

  @io.grpc.stub.annotations.RpcMethod(
      fullMethodName = SERVICE_NAME + '/' + "RegisterModel",
      requestType = omniharness.v1.RegisterModelReq.class,
      responseType = omniharness.v1.RegisterModelResp.class,
      methodType = io.grpc.MethodDescriptor.MethodType.UNARY)
  public static io.grpc.MethodDescriptor<omniharness.v1.RegisterModelReq,
      omniharness.v1.RegisterModelResp> getRegisterModelMethod() {
    io.grpc.MethodDescriptor<omniharness.v1.RegisterModelReq, omniharness.v1.RegisterModelResp> getRegisterModelMethod;
    if ((getRegisterModelMethod = ModelServiceGrpc.getRegisterModelMethod) == null) {
      synchronized (ModelServiceGrpc.class) {
        if ((getRegisterModelMethod = ModelServiceGrpc.getRegisterModelMethod) == null) {
          ModelServiceGrpc.getRegisterModelMethod = getRegisterModelMethod =
              io.grpc.MethodDescriptor.<omniharness.v1.RegisterModelReq, omniharness.v1.RegisterModelResp>newBuilder()
              .setType(io.grpc.MethodDescriptor.MethodType.UNARY)
              .setFullMethodName(generateFullMethodName(SERVICE_NAME, "RegisterModel"))
              .setSampledToLocalTracing(true)
              .setRequestMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  omniharness.v1.RegisterModelReq.getDefaultInstance()))
              .setResponseMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  omniharness.v1.RegisterModelResp.getDefaultInstance()))
              .setSchemaDescriptor(new ModelServiceMethodDescriptorSupplier("RegisterModel"))
              .build();
        }
      }
    }
    return getRegisterModelMethod;
  }

  /**
   * Creates a new async stub that supports all call types for the service
   */
  public static ModelServiceStub newStub(io.grpc.Channel channel) {
    io.grpc.stub.AbstractStub.StubFactory<ModelServiceStub> factory =
      new io.grpc.stub.AbstractStub.StubFactory<ModelServiceStub>() {
        @java.lang.Override
        public ModelServiceStub newStub(io.grpc.Channel channel, io.grpc.CallOptions callOptions) {
          return new ModelServiceStub(channel, callOptions);
        }
      };
    return ModelServiceStub.newStub(factory, channel);
  }

  /**
   * Creates a new blocking-style stub that supports unary and streaming output calls on the service
   */
  public static ModelServiceBlockingStub newBlockingStub(
      io.grpc.Channel channel) {
    io.grpc.stub.AbstractStub.StubFactory<ModelServiceBlockingStub> factory =
      new io.grpc.stub.AbstractStub.StubFactory<ModelServiceBlockingStub>() {
        @java.lang.Override
        public ModelServiceBlockingStub newStub(io.grpc.Channel channel, io.grpc.CallOptions callOptions) {
          return new ModelServiceBlockingStub(channel, callOptions);
        }
      };
    return ModelServiceBlockingStub.newStub(factory, channel);
  }

  /**
   * Creates a new ListenableFuture-style stub that supports unary calls on the service
   */
  public static ModelServiceFutureStub newFutureStub(
      io.grpc.Channel channel) {
    io.grpc.stub.AbstractStub.StubFactory<ModelServiceFutureStub> factory =
      new io.grpc.stub.AbstractStub.StubFactory<ModelServiceFutureStub>() {
        @java.lang.Override
        public ModelServiceFutureStub newStub(io.grpc.Channel channel, io.grpc.CallOptions callOptions) {
          return new ModelServiceFutureStub(channel, callOptions);
        }
      };
    return ModelServiceFutureStub.newStub(factory, channel);
  }

  /**
   */
  public interface AsyncService {

    /**
     */
    default void chat(omniharness.v1.ChatRequest request,
        io.grpc.stub.StreamObserver<omniharness.v1.ChatResponse> responseObserver) {
      io.grpc.stub.ServerCalls.asyncUnimplementedUnaryCall(getChatMethod(), responseObserver);
    }

    /**
     */
    default void chatStream(omniharness.v1.ChatRequest request,
        io.grpc.stub.StreamObserver<omniharness.v1.ChatChunk> responseObserver) {
      io.grpc.stub.ServerCalls.asyncUnimplementedUnaryCall(getChatStreamMethod(), responseObserver);
    }

    /**
     */
    default void listModels(omniharness.v1.ListModelsRequest request,
        io.grpc.stub.StreamObserver<omniharness.v1.ListModelsResponse> responseObserver) {
      io.grpc.stub.ServerCalls.asyncUnimplementedUnaryCall(getListModelsMethod(), responseObserver);
    }

    /**
     */
    default void healthCheck(omniharness.v1.ModelHealthRequest request,
        io.grpc.stub.StreamObserver<omniharness.v1.ModelHealthResponse> responseObserver) {
      io.grpc.stub.ServerCalls.asyncUnimplementedUnaryCall(getHealthCheckMethod(), responseObserver);
    }

    /**
     */
    default void registerModel(omniharness.v1.RegisterModelReq request,
        io.grpc.stub.StreamObserver<omniharness.v1.RegisterModelResp> responseObserver) {
      io.grpc.stub.ServerCalls.asyncUnimplementedUnaryCall(getRegisterModelMethod(), responseObserver);
    }
  }

  /**
   * Base class for the server implementation of the service ModelService.
   */
  public static abstract class ModelServiceImplBase
      implements io.grpc.BindableService, AsyncService {

    @java.lang.Override public final io.grpc.ServerServiceDefinition bindService() {
      return ModelServiceGrpc.bindService(this);
    }
  }

  /**
   * A stub to allow clients to do asynchronous rpc calls to service ModelService.
   */
  public static final class ModelServiceStub
      extends io.grpc.stub.AbstractAsyncStub<ModelServiceStub> {
    private ModelServiceStub(
        io.grpc.Channel channel, io.grpc.CallOptions callOptions) {
      super(channel, callOptions);
    }

    @java.lang.Override
    protected ModelServiceStub build(
        io.grpc.Channel channel, io.grpc.CallOptions callOptions) {
      return new ModelServiceStub(channel, callOptions);
    }

    /**
     */
    public void chat(omniharness.v1.ChatRequest request,
        io.grpc.stub.StreamObserver<omniharness.v1.ChatResponse> responseObserver) {
      io.grpc.stub.ClientCalls.asyncUnaryCall(
          getChannel().newCall(getChatMethod(), getCallOptions()), request, responseObserver);
    }

    /**
     */
    public void chatStream(omniharness.v1.ChatRequest request,
        io.grpc.stub.StreamObserver<omniharness.v1.ChatChunk> responseObserver) {
      io.grpc.stub.ClientCalls.asyncServerStreamingCall(
          getChannel().newCall(getChatStreamMethod(), getCallOptions()), request, responseObserver);
    }

    /**
     */
    public void listModels(omniharness.v1.ListModelsRequest request,
        io.grpc.stub.StreamObserver<omniharness.v1.ListModelsResponse> responseObserver) {
      io.grpc.stub.ClientCalls.asyncUnaryCall(
          getChannel().newCall(getListModelsMethod(), getCallOptions()), request, responseObserver);
    }

    /**
     */
    public void healthCheck(omniharness.v1.ModelHealthRequest request,
        io.grpc.stub.StreamObserver<omniharness.v1.ModelHealthResponse> responseObserver) {
      io.grpc.stub.ClientCalls.asyncUnaryCall(
          getChannel().newCall(getHealthCheckMethod(), getCallOptions()), request, responseObserver);
    }

    /**
     */
    public void registerModel(omniharness.v1.RegisterModelReq request,
        io.grpc.stub.StreamObserver<omniharness.v1.RegisterModelResp> responseObserver) {
      io.grpc.stub.ClientCalls.asyncUnaryCall(
          getChannel().newCall(getRegisterModelMethod(), getCallOptions()), request, responseObserver);
    }
  }

  /**
   * A stub to allow clients to do synchronous rpc calls to service ModelService.
   */
  public static final class ModelServiceBlockingStub
      extends io.grpc.stub.AbstractBlockingStub<ModelServiceBlockingStub> {
    private ModelServiceBlockingStub(
        io.grpc.Channel channel, io.grpc.CallOptions callOptions) {
      super(channel, callOptions);
    }

    @java.lang.Override
    protected ModelServiceBlockingStub build(
        io.grpc.Channel channel, io.grpc.CallOptions callOptions) {
      return new ModelServiceBlockingStub(channel, callOptions);
    }

    /**
     */
    public omniharness.v1.ChatResponse chat(omniharness.v1.ChatRequest request) {
      return io.grpc.stub.ClientCalls.blockingUnaryCall(
          getChannel(), getChatMethod(), getCallOptions(), request);
    }

    /**
     */
    public java.util.Iterator<omniharness.v1.ChatChunk> chatStream(
        omniharness.v1.ChatRequest request) {
      return io.grpc.stub.ClientCalls.blockingServerStreamingCall(
          getChannel(), getChatStreamMethod(), getCallOptions(), request);
    }

    /**
     */
    public omniharness.v1.ListModelsResponse listModels(omniharness.v1.ListModelsRequest request) {
      return io.grpc.stub.ClientCalls.blockingUnaryCall(
          getChannel(), getListModelsMethod(), getCallOptions(), request);
    }

    /**
     */
    public omniharness.v1.ModelHealthResponse healthCheck(omniharness.v1.ModelHealthRequest request) {
      return io.grpc.stub.ClientCalls.blockingUnaryCall(
          getChannel(), getHealthCheckMethod(), getCallOptions(), request);
    }

    /**
     */
    public omniharness.v1.RegisterModelResp registerModel(omniharness.v1.RegisterModelReq request) {
      return io.grpc.stub.ClientCalls.blockingUnaryCall(
          getChannel(), getRegisterModelMethod(), getCallOptions(), request);
    }
  }

  /**
   * A stub to allow clients to do ListenableFuture-style rpc calls to service ModelService.
   */
  public static final class ModelServiceFutureStub
      extends io.grpc.stub.AbstractFutureStub<ModelServiceFutureStub> {
    private ModelServiceFutureStub(
        io.grpc.Channel channel, io.grpc.CallOptions callOptions) {
      super(channel, callOptions);
    }

    @java.lang.Override
    protected ModelServiceFutureStub build(
        io.grpc.Channel channel, io.grpc.CallOptions callOptions) {
      return new ModelServiceFutureStub(channel, callOptions);
    }

    /**
     */
    public com.google.common.util.concurrent.ListenableFuture<omniharness.v1.ChatResponse> chat(
        omniharness.v1.ChatRequest request) {
      return io.grpc.stub.ClientCalls.futureUnaryCall(
          getChannel().newCall(getChatMethod(), getCallOptions()), request);
    }

    /**
     */
    public com.google.common.util.concurrent.ListenableFuture<omniharness.v1.ListModelsResponse> listModels(
        omniharness.v1.ListModelsRequest request) {
      return io.grpc.stub.ClientCalls.futureUnaryCall(
          getChannel().newCall(getListModelsMethod(), getCallOptions()), request);
    }

    /**
     */
    public com.google.common.util.concurrent.ListenableFuture<omniharness.v1.ModelHealthResponse> healthCheck(
        omniharness.v1.ModelHealthRequest request) {
      return io.grpc.stub.ClientCalls.futureUnaryCall(
          getChannel().newCall(getHealthCheckMethod(), getCallOptions()), request);
    }

    /**
     */
    public com.google.common.util.concurrent.ListenableFuture<omniharness.v1.RegisterModelResp> registerModel(
        omniharness.v1.RegisterModelReq request) {
      return io.grpc.stub.ClientCalls.futureUnaryCall(
          getChannel().newCall(getRegisterModelMethod(), getCallOptions()), request);
    }
  }

  private static final int METHODID_CHAT = 0;
  private static final int METHODID_CHAT_STREAM = 1;
  private static final int METHODID_LIST_MODELS = 2;
  private static final int METHODID_HEALTH_CHECK = 3;
  private static final int METHODID_REGISTER_MODEL = 4;

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
        case METHODID_CHAT:
          serviceImpl.chat((omniharness.v1.ChatRequest) request,
              (io.grpc.stub.StreamObserver<omniharness.v1.ChatResponse>) responseObserver);
          break;
        case METHODID_CHAT_STREAM:
          serviceImpl.chatStream((omniharness.v1.ChatRequest) request,
              (io.grpc.stub.StreamObserver<omniharness.v1.ChatChunk>) responseObserver);
          break;
        case METHODID_LIST_MODELS:
          serviceImpl.listModels((omniharness.v1.ListModelsRequest) request,
              (io.grpc.stub.StreamObserver<omniharness.v1.ListModelsResponse>) responseObserver);
          break;
        case METHODID_HEALTH_CHECK:
          serviceImpl.healthCheck((omniharness.v1.ModelHealthRequest) request,
              (io.grpc.stub.StreamObserver<omniharness.v1.ModelHealthResponse>) responseObserver);
          break;
        case METHODID_REGISTER_MODEL:
          serviceImpl.registerModel((omniharness.v1.RegisterModelReq) request,
              (io.grpc.stub.StreamObserver<omniharness.v1.RegisterModelResp>) responseObserver);
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
          getChatMethod(),
          io.grpc.stub.ServerCalls.asyncUnaryCall(
            new MethodHandlers<
              omniharness.v1.ChatRequest,
              omniharness.v1.ChatResponse>(
                service, METHODID_CHAT)))
        .addMethod(
          getChatStreamMethod(),
          io.grpc.stub.ServerCalls.asyncServerStreamingCall(
            new MethodHandlers<
              omniharness.v1.ChatRequest,
              omniharness.v1.ChatChunk>(
                service, METHODID_CHAT_STREAM)))
        .addMethod(
          getListModelsMethod(),
          io.grpc.stub.ServerCalls.asyncUnaryCall(
            new MethodHandlers<
              omniharness.v1.ListModelsRequest,
              omniharness.v1.ListModelsResponse>(
                service, METHODID_LIST_MODELS)))
        .addMethod(
          getHealthCheckMethod(),
          io.grpc.stub.ServerCalls.asyncUnaryCall(
            new MethodHandlers<
              omniharness.v1.ModelHealthRequest,
              omniharness.v1.ModelHealthResponse>(
                service, METHODID_HEALTH_CHECK)))
        .addMethod(
          getRegisterModelMethod(),
          io.grpc.stub.ServerCalls.asyncUnaryCall(
            new MethodHandlers<
              omniharness.v1.RegisterModelReq,
              omniharness.v1.RegisterModelResp>(
                service, METHODID_REGISTER_MODEL)))
        .build();
  }

  private static abstract class ModelServiceBaseDescriptorSupplier
      implements io.grpc.protobuf.ProtoFileDescriptorSupplier, io.grpc.protobuf.ProtoServiceDescriptorSupplier {
    ModelServiceBaseDescriptorSupplier() {}

    @java.lang.Override
    public com.google.protobuf.Descriptors.FileDescriptor getFileDescriptor() {
      return omniharness.v1.Omniharness.getDescriptor();
    }

    @java.lang.Override
    public com.google.protobuf.Descriptors.ServiceDescriptor getServiceDescriptor() {
      return getFileDescriptor().findServiceByName("ModelService");
    }
  }

  private static final class ModelServiceFileDescriptorSupplier
      extends ModelServiceBaseDescriptorSupplier {
    ModelServiceFileDescriptorSupplier() {}
  }

  private static final class ModelServiceMethodDescriptorSupplier
      extends ModelServiceBaseDescriptorSupplier
      implements io.grpc.protobuf.ProtoMethodDescriptorSupplier {
    private final java.lang.String methodName;

    ModelServiceMethodDescriptorSupplier(java.lang.String methodName) {
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
      synchronized (ModelServiceGrpc.class) {
        result = serviceDescriptor;
        if (result == null) {
          serviceDescriptor = result = io.grpc.ServiceDescriptor.newBuilder(SERVICE_NAME)
              .setSchemaDescriptor(new ModelServiceFileDescriptorSupplier())
              .addMethod(getChatMethod())
              .addMethod(getChatStreamMethod())
              .addMethod(getListModelsMethod())
              .addMethod(getHealthCheckMethod())
              .addMethod(getRegisterModelMethod())
              .build();
        }
      }
    }
    return result;
  }
}
