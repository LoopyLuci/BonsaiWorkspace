package omniharness.v1;

import static io.grpc.MethodDescriptor.generateFullMethodName;

/**
 */
@javax.annotation.Generated(
    value = "by gRPC proto compiler (version 1.68.1)",
    comments = "Source: omniharness.proto")
@io.grpc.stub.annotations.GrpcGenerated
public final class MeshServiceGrpc {

  private MeshServiceGrpc() {}

  public static final java.lang.String SERVICE_NAME = "omniharness.v1.MeshService";

  // Static method descriptors that strictly reflect the proto.
  private static volatile io.grpc.MethodDescriptor<omniharness.v1.BroadcastRequest,
      omniharness.v1.BroadcastResponse> getBroadcastEventMethod;

  @io.grpc.stub.annotations.RpcMethod(
      fullMethodName = SERVICE_NAME + '/' + "BroadcastEvent",
      requestType = omniharness.v1.BroadcastRequest.class,
      responseType = omniharness.v1.BroadcastResponse.class,
      methodType = io.grpc.MethodDescriptor.MethodType.UNARY)
  public static io.grpc.MethodDescriptor<omniharness.v1.BroadcastRequest,
      omniharness.v1.BroadcastResponse> getBroadcastEventMethod() {
    io.grpc.MethodDescriptor<omniharness.v1.BroadcastRequest, omniharness.v1.BroadcastResponse> getBroadcastEventMethod;
    if ((getBroadcastEventMethod = MeshServiceGrpc.getBroadcastEventMethod) == null) {
      synchronized (MeshServiceGrpc.class) {
        if ((getBroadcastEventMethod = MeshServiceGrpc.getBroadcastEventMethod) == null) {
          MeshServiceGrpc.getBroadcastEventMethod = getBroadcastEventMethod =
              io.grpc.MethodDescriptor.<omniharness.v1.BroadcastRequest, omniharness.v1.BroadcastResponse>newBuilder()
              .setType(io.grpc.MethodDescriptor.MethodType.UNARY)
              .setFullMethodName(generateFullMethodName(SERVICE_NAME, "BroadcastEvent"))
              .setSampledToLocalTracing(true)
              .setRequestMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  omniharness.v1.BroadcastRequest.getDefaultInstance()))
              .setResponseMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  omniharness.v1.BroadcastResponse.getDefaultInstance()))
              .setSchemaDescriptor(new MeshServiceMethodDescriptorSupplier("BroadcastEvent"))
              .build();
        }
      }
    }
    return getBroadcastEventMethod;
  }

  private static volatile io.grpc.MethodDescriptor<omniharness.v1.ListPeersRequest,
      omniharness.v1.ListPeersResponse> getListPeersMethod;

  @io.grpc.stub.annotations.RpcMethod(
      fullMethodName = SERVICE_NAME + '/' + "ListPeers",
      requestType = omniharness.v1.ListPeersRequest.class,
      responseType = omniharness.v1.ListPeersResponse.class,
      methodType = io.grpc.MethodDescriptor.MethodType.UNARY)
  public static io.grpc.MethodDescriptor<omniharness.v1.ListPeersRequest,
      omniharness.v1.ListPeersResponse> getListPeersMethod() {
    io.grpc.MethodDescriptor<omniharness.v1.ListPeersRequest, omniharness.v1.ListPeersResponse> getListPeersMethod;
    if ((getListPeersMethod = MeshServiceGrpc.getListPeersMethod) == null) {
      synchronized (MeshServiceGrpc.class) {
        if ((getListPeersMethod = MeshServiceGrpc.getListPeersMethod) == null) {
          MeshServiceGrpc.getListPeersMethod = getListPeersMethod =
              io.grpc.MethodDescriptor.<omniharness.v1.ListPeersRequest, omniharness.v1.ListPeersResponse>newBuilder()
              .setType(io.grpc.MethodDescriptor.MethodType.UNARY)
              .setFullMethodName(generateFullMethodName(SERVICE_NAME, "ListPeers"))
              .setSampledToLocalTracing(true)
              .setRequestMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  omniharness.v1.ListPeersRequest.getDefaultInstance()))
              .setResponseMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  omniharness.v1.ListPeersResponse.getDefaultInstance()))
              .setSchemaDescriptor(new MeshServiceMethodDescriptorSupplier("ListPeers"))
              .build();
        }
      }
    }
    return getListPeersMethod;
  }

  /**
   * Creates a new async stub that supports all call types for the service
   */
  public static MeshServiceStub newStub(io.grpc.Channel channel) {
    io.grpc.stub.AbstractStub.StubFactory<MeshServiceStub> factory =
      new io.grpc.stub.AbstractStub.StubFactory<MeshServiceStub>() {
        @java.lang.Override
        public MeshServiceStub newStub(io.grpc.Channel channel, io.grpc.CallOptions callOptions) {
          return new MeshServiceStub(channel, callOptions);
        }
      };
    return MeshServiceStub.newStub(factory, channel);
  }

  /**
   * Creates a new blocking-style stub that supports unary and streaming output calls on the service
   */
  public static MeshServiceBlockingStub newBlockingStub(
      io.grpc.Channel channel) {
    io.grpc.stub.AbstractStub.StubFactory<MeshServiceBlockingStub> factory =
      new io.grpc.stub.AbstractStub.StubFactory<MeshServiceBlockingStub>() {
        @java.lang.Override
        public MeshServiceBlockingStub newStub(io.grpc.Channel channel, io.grpc.CallOptions callOptions) {
          return new MeshServiceBlockingStub(channel, callOptions);
        }
      };
    return MeshServiceBlockingStub.newStub(factory, channel);
  }

  /**
   * Creates a new ListenableFuture-style stub that supports unary calls on the service
   */
  public static MeshServiceFutureStub newFutureStub(
      io.grpc.Channel channel) {
    io.grpc.stub.AbstractStub.StubFactory<MeshServiceFutureStub> factory =
      new io.grpc.stub.AbstractStub.StubFactory<MeshServiceFutureStub>() {
        @java.lang.Override
        public MeshServiceFutureStub newStub(io.grpc.Channel channel, io.grpc.CallOptions callOptions) {
          return new MeshServiceFutureStub(channel, callOptions);
        }
      };
    return MeshServiceFutureStub.newStub(factory, channel);
  }

  /**
   */
  public interface AsyncService {

    /**
     */
    default void broadcastEvent(omniharness.v1.BroadcastRequest request,
        io.grpc.stub.StreamObserver<omniharness.v1.BroadcastResponse> responseObserver) {
      io.grpc.stub.ServerCalls.asyncUnimplementedUnaryCall(getBroadcastEventMethod(), responseObserver);
    }

    /**
     */
    default void listPeers(omniharness.v1.ListPeersRequest request,
        io.grpc.stub.StreamObserver<omniharness.v1.ListPeersResponse> responseObserver) {
      io.grpc.stub.ServerCalls.asyncUnimplementedUnaryCall(getListPeersMethod(), responseObserver);
    }
  }

  /**
   * Base class for the server implementation of the service MeshService.
   */
  public static abstract class MeshServiceImplBase
      implements io.grpc.BindableService, AsyncService {

    @java.lang.Override public final io.grpc.ServerServiceDefinition bindService() {
      return MeshServiceGrpc.bindService(this);
    }
  }

  /**
   * A stub to allow clients to do asynchronous rpc calls to service MeshService.
   */
  public static final class MeshServiceStub
      extends io.grpc.stub.AbstractAsyncStub<MeshServiceStub> {
    private MeshServiceStub(
        io.grpc.Channel channel, io.grpc.CallOptions callOptions) {
      super(channel, callOptions);
    }

    @java.lang.Override
    protected MeshServiceStub build(
        io.grpc.Channel channel, io.grpc.CallOptions callOptions) {
      return new MeshServiceStub(channel, callOptions);
    }

    /**
     */
    public void broadcastEvent(omniharness.v1.BroadcastRequest request,
        io.grpc.stub.StreamObserver<omniharness.v1.BroadcastResponse> responseObserver) {
      io.grpc.stub.ClientCalls.asyncUnaryCall(
          getChannel().newCall(getBroadcastEventMethod(), getCallOptions()), request, responseObserver);
    }

    /**
     */
    public void listPeers(omniharness.v1.ListPeersRequest request,
        io.grpc.stub.StreamObserver<omniharness.v1.ListPeersResponse> responseObserver) {
      io.grpc.stub.ClientCalls.asyncUnaryCall(
          getChannel().newCall(getListPeersMethod(), getCallOptions()), request, responseObserver);
    }
  }

  /**
   * A stub to allow clients to do synchronous rpc calls to service MeshService.
   */
  public static final class MeshServiceBlockingStub
      extends io.grpc.stub.AbstractBlockingStub<MeshServiceBlockingStub> {
    private MeshServiceBlockingStub(
        io.grpc.Channel channel, io.grpc.CallOptions callOptions) {
      super(channel, callOptions);
    }

    @java.lang.Override
    protected MeshServiceBlockingStub build(
        io.grpc.Channel channel, io.grpc.CallOptions callOptions) {
      return new MeshServiceBlockingStub(channel, callOptions);
    }

    /**
     */
    public omniharness.v1.BroadcastResponse broadcastEvent(omniharness.v1.BroadcastRequest request) {
      return io.grpc.stub.ClientCalls.blockingUnaryCall(
          getChannel(), getBroadcastEventMethod(), getCallOptions(), request);
    }

    /**
     */
    public omniharness.v1.ListPeersResponse listPeers(omniharness.v1.ListPeersRequest request) {
      return io.grpc.stub.ClientCalls.blockingUnaryCall(
          getChannel(), getListPeersMethod(), getCallOptions(), request);
    }
  }

  /**
   * A stub to allow clients to do ListenableFuture-style rpc calls to service MeshService.
   */
  public static final class MeshServiceFutureStub
      extends io.grpc.stub.AbstractFutureStub<MeshServiceFutureStub> {
    private MeshServiceFutureStub(
        io.grpc.Channel channel, io.grpc.CallOptions callOptions) {
      super(channel, callOptions);
    }

    @java.lang.Override
    protected MeshServiceFutureStub build(
        io.grpc.Channel channel, io.grpc.CallOptions callOptions) {
      return new MeshServiceFutureStub(channel, callOptions);
    }

    /**
     */
    public com.google.common.util.concurrent.ListenableFuture<omniharness.v1.BroadcastResponse> broadcastEvent(
        omniharness.v1.BroadcastRequest request) {
      return io.grpc.stub.ClientCalls.futureUnaryCall(
          getChannel().newCall(getBroadcastEventMethod(), getCallOptions()), request);
    }

    /**
     */
    public com.google.common.util.concurrent.ListenableFuture<omniharness.v1.ListPeersResponse> listPeers(
        omniharness.v1.ListPeersRequest request) {
      return io.grpc.stub.ClientCalls.futureUnaryCall(
          getChannel().newCall(getListPeersMethod(), getCallOptions()), request);
    }
  }

  private static final int METHODID_BROADCAST_EVENT = 0;
  private static final int METHODID_LIST_PEERS = 1;

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
        case METHODID_BROADCAST_EVENT:
          serviceImpl.broadcastEvent((omniharness.v1.BroadcastRequest) request,
              (io.grpc.stub.StreamObserver<omniharness.v1.BroadcastResponse>) responseObserver);
          break;
        case METHODID_LIST_PEERS:
          serviceImpl.listPeers((omniharness.v1.ListPeersRequest) request,
              (io.grpc.stub.StreamObserver<omniharness.v1.ListPeersResponse>) responseObserver);
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
          getBroadcastEventMethod(),
          io.grpc.stub.ServerCalls.asyncUnaryCall(
            new MethodHandlers<
              omniharness.v1.BroadcastRequest,
              omniharness.v1.BroadcastResponse>(
                service, METHODID_BROADCAST_EVENT)))
        .addMethod(
          getListPeersMethod(),
          io.grpc.stub.ServerCalls.asyncUnaryCall(
            new MethodHandlers<
              omniharness.v1.ListPeersRequest,
              omniharness.v1.ListPeersResponse>(
                service, METHODID_LIST_PEERS)))
        .build();
  }

  private static abstract class MeshServiceBaseDescriptorSupplier
      implements io.grpc.protobuf.ProtoFileDescriptorSupplier, io.grpc.protobuf.ProtoServiceDescriptorSupplier {
    MeshServiceBaseDescriptorSupplier() {}

    @java.lang.Override
    public com.google.protobuf.Descriptors.FileDescriptor getFileDescriptor() {
      return omniharness.v1.Omniharness.getDescriptor();
    }

    @java.lang.Override
    public com.google.protobuf.Descriptors.ServiceDescriptor getServiceDescriptor() {
      return getFileDescriptor().findServiceByName("MeshService");
    }
  }

  private static final class MeshServiceFileDescriptorSupplier
      extends MeshServiceBaseDescriptorSupplier {
    MeshServiceFileDescriptorSupplier() {}
  }

  private static final class MeshServiceMethodDescriptorSupplier
      extends MeshServiceBaseDescriptorSupplier
      implements io.grpc.protobuf.ProtoMethodDescriptorSupplier {
    private final java.lang.String methodName;

    MeshServiceMethodDescriptorSupplier(java.lang.String methodName) {
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
      synchronized (MeshServiceGrpc.class) {
        result = serviceDescriptor;
        if (result == null) {
          serviceDescriptor = result = io.grpc.ServiceDescriptor.newBuilder(SERVICE_NAME)
              .setSchemaDescriptor(new MeshServiceFileDescriptorSupplier())
              .addMethod(getBroadcastEventMethod())
              .addMethod(getListPeersMethod())
              .build();
        }
      }
    }
    return result;
  }
}
