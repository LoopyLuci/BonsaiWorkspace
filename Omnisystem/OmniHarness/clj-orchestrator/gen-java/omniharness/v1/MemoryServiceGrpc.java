package omniharness.v1;

import static io.grpc.MethodDescriptor.generateFullMethodName;

/**
 */
@javax.annotation.Generated(
    value = "by gRPC proto compiler (version 1.68.1)",
    comments = "Source: omniharness.proto")
@io.grpc.stub.annotations.GrpcGenerated
public final class MemoryServiceGrpc {

  private MemoryServiceGrpc() {}

  public static final java.lang.String SERVICE_NAME = "omniharness.v1.MemoryService";

  // Static method descriptors that strictly reflect the proto.
  private static volatile io.grpc.MethodDescriptor<omniharness.v1.StoreRequest,
      omniharness.v1.StoreResponse> getStoreMethod;

  @io.grpc.stub.annotations.RpcMethod(
      fullMethodName = SERVICE_NAME + '/' + "Store",
      requestType = omniharness.v1.StoreRequest.class,
      responseType = omniharness.v1.StoreResponse.class,
      methodType = io.grpc.MethodDescriptor.MethodType.UNARY)
  public static io.grpc.MethodDescriptor<omniharness.v1.StoreRequest,
      omniharness.v1.StoreResponse> getStoreMethod() {
    io.grpc.MethodDescriptor<omniharness.v1.StoreRequest, omniharness.v1.StoreResponse> getStoreMethod;
    if ((getStoreMethod = MemoryServiceGrpc.getStoreMethod) == null) {
      synchronized (MemoryServiceGrpc.class) {
        if ((getStoreMethod = MemoryServiceGrpc.getStoreMethod) == null) {
          MemoryServiceGrpc.getStoreMethod = getStoreMethod =
              io.grpc.MethodDescriptor.<omniharness.v1.StoreRequest, omniharness.v1.StoreResponse>newBuilder()
              .setType(io.grpc.MethodDescriptor.MethodType.UNARY)
              .setFullMethodName(generateFullMethodName(SERVICE_NAME, "Store"))
              .setSampledToLocalTracing(true)
              .setRequestMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  omniharness.v1.StoreRequest.getDefaultInstance()))
              .setResponseMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  omniharness.v1.StoreResponse.getDefaultInstance()))
              .setSchemaDescriptor(new MemoryServiceMethodDescriptorSupplier("Store"))
              .build();
        }
      }
    }
    return getStoreMethod;
  }

  private static volatile io.grpc.MethodDescriptor<omniharness.v1.RetrieveRequest,
      omniharness.v1.RetrieveResponse> getRetrieveMethod;

  @io.grpc.stub.annotations.RpcMethod(
      fullMethodName = SERVICE_NAME + '/' + "Retrieve",
      requestType = omniharness.v1.RetrieveRequest.class,
      responseType = omniharness.v1.RetrieveResponse.class,
      methodType = io.grpc.MethodDescriptor.MethodType.UNARY)
  public static io.grpc.MethodDescriptor<omniharness.v1.RetrieveRequest,
      omniharness.v1.RetrieveResponse> getRetrieveMethod() {
    io.grpc.MethodDescriptor<omniharness.v1.RetrieveRequest, omniharness.v1.RetrieveResponse> getRetrieveMethod;
    if ((getRetrieveMethod = MemoryServiceGrpc.getRetrieveMethod) == null) {
      synchronized (MemoryServiceGrpc.class) {
        if ((getRetrieveMethod = MemoryServiceGrpc.getRetrieveMethod) == null) {
          MemoryServiceGrpc.getRetrieveMethod = getRetrieveMethod =
              io.grpc.MethodDescriptor.<omniharness.v1.RetrieveRequest, omniharness.v1.RetrieveResponse>newBuilder()
              .setType(io.grpc.MethodDescriptor.MethodType.UNARY)
              .setFullMethodName(generateFullMethodName(SERVICE_NAME, "Retrieve"))
              .setSampledToLocalTracing(true)
              .setRequestMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  omniharness.v1.RetrieveRequest.getDefaultInstance()))
              .setResponseMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  omniharness.v1.RetrieveResponse.getDefaultInstance()))
              .setSchemaDescriptor(new MemoryServiceMethodDescriptorSupplier("Retrieve"))
              .build();
        }
      }
    }
    return getRetrieveMethod;
  }

  private static volatile io.grpc.MethodDescriptor<omniharness.v1.SemanticSearchRequest,
      omniharness.v1.SemanticSearchResponse> getSearchSemanticMethod;

  @io.grpc.stub.annotations.RpcMethod(
      fullMethodName = SERVICE_NAME + '/' + "SearchSemantic",
      requestType = omniharness.v1.SemanticSearchRequest.class,
      responseType = omniharness.v1.SemanticSearchResponse.class,
      methodType = io.grpc.MethodDescriptor.MethodType.UNARY)
  public static io.grpc.MethodDescriptor<omniharness.v1.SemanticSearchRequest,
      omniharness.v1.SemanticSearchResponse> getSearchSemanticMethod() {
    io.grpc.MethodDescriptor<omniharness.v1.SemanticSearchRequest, omniharness.v1.SemanticSearchResponse> getSearchSemanticMethod;
    if ((getSearchSemanticMethod = MemoryServiceGrpc.getSearchSemanticMethod) == null) {
      synchronized (MemoryServiceGrpc.class) {
        if ((getSearchSemanticMethod = MemoryServiceGrpc.getSearchSemanticMethod) == null) {
          MemoryServiceGrpc.getSearchSemanticMethod = getSearchSemanticMethod =
              io.grpc.MethodDescriptor.<omniharness.v1.SemanticSearchRequest, omniharness.v1.SemanticSearchResponse>newBuilder()
              .setType(io.grpc.MethodDescriptor.MethodType.UNARY)
              .setFullMethodName(generateFullMethodName(SERVICE_NAME, "SearchSemantic"))
              .setSampledToLocalTracing(true)
              .setRequestMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  omniharness.v1.SemanticSearchRequest.getDefaultInstance()))
              .setResponseMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  omniharness.v1.SemanticSearchResponse.getDefaultInstance()))
              .setSchemaDescriptor(new MemoryServiceMethodDescriptorSupplier("SearchSemantic"))
              .build();
        }
      }
    }
    return getSearchSemanticMethod;
  }

  private static volatile io.grpc.MethodDescriptor<omniharness.v1.DeleteRequest,
      omniharness.v1.DeleteResponse> getDeleteMethod;

  @io.grpc.stub.annotations.RpcMethod(
      fullMethodName = SERVICE_NAME + '/' + "Delete",
      requestType = omniharness.v1.DeleteRequest.class,
      responseType = omniharness.v1.DeleteResponse.class,
      methodType = io.grpc.MethodDescriptor.MethodType.UNARY)
  public static io.grpc.MethodDescriptor<omniharness.v1.DeleteRequest,
      omniharness.v1.DeleteResponse> getDeleteMethod() {
    io.grpc.MethodDescriptor<omniharness.v1.DeleteRequest, omniharness.v1.DeleteResponse> getDeleteMethod;
    if ((getDeleteMethod = MemoryServiceGrpc.getDeleteMethod) == null) {
      synchronized (MemoryServiceGrpc.class) {
        if ((getDeleteMethod = MemoryServiceGrpc.getDeleteMethod) == null) {
          MemoryServiceGrpc.getDeleteMethod = getDeleteMethod =
              io.grpc.MethodDescriptor.<omniharness.v1.DeleteRequest, omniharness.v1.DeleteResponse>newBuilder()
              .setType(io.grpc.MethodDescriptor.MethodType.UNARY)
              .setFullMethodName(generateFullMethodName(SERVICE_NAME, "Delete"))
              .setSampledToLocalTracing(true)
              .setRequestMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  omniharness.v1.DeleteRequest.getDefaultInstance()))
              .setResponseMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  omniharness.v1.DeleteResponse.getDefaultInstance()))
              .setSchemaDescriptor(new MemoryServiceMethodDescriptorSupplier("Delete"))
              .build();
        }
      }
    }
    return getDeleteMethod;
  }

  private static volatile io.grpc.MethodDescriptor<omniharness.v1.ListCollRequest,
      omniharness.v1.ListCollResponse> getListCollectionsMethod;

  @io.grpc.stub.annotations.RpcMethod(
      fullMethodName = SERVICE_NAME + '/' + "ListCollections",
      requestType = omniharness.v1.ListCollRequest.class,
      responseType = omniharness.v1.ListCollResponse.class,
      methodType = io.grpc.MethodDescriptor.MethodType.UNARY)
  public static io.grpc.MethodDescriptor<omniharness.v1.ListCollRequest,
      omniharness.v1.ListCollResponse> getListCollectionsMethod() {
    io.grpc.MethodDescriptor<omniharness.v1.ListCollRequest, omniharness.v1.ListCollResponse> getListCollectionsMethod;
    if ((getListCollectionsMethod = MemoryServiceGrpc.getListCollectionsMethod) == null) {
      synchronized (MemoryServiceGrpc.class) {
        if ((getListCollectionsMethod = MemoryServiceGrpc.getListCollectionsMethod) == null) {
          MemoryServiceGrpc.getListCollectionsMethod = getListCollectionsMethod =
              io.grpc.MethodDescriptor.<omniharness.v1.ListCollRequest, omniharness.v1.ListCollResponse>newBuilder()
              .setType(io.grpc.MethodDescriptor.MethodType.UNARY)
              .setFullMethodName(generateFullMethodName(SERVICE_NAME, "ListCollections"))
              .setSampledToLocalTracing(true)
              .setRequestMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  omniharness.v1.ListCollRequest.getDefaultInstance()))
              .setResponseMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  omniharness.v1.ListCollResponse.getDefaultInstance()))
              .setSchemaDescriptor(new MemoryServiceMethodDescriptorSupplier("ListCollections"))
              .build();
        }
      }
    }
    return getListCollectionsMethod;
  }

  private static volatile io.grpc.MethodDescriptor<omniharness.v1.SummarizeRequest,
      omniharness.v1.SummarizeResponse> getSummarizeMethod;

  @io.grpc.stub.annotations.RpcMethod(
      fullMethodName = SERVICE_NAME + '/' + "Summarize",
      requestType = omniharness.v1.SummarizeRequest.class,
      responseType = omniharness.v1.SummarizeResponse.class,
      methodType = io.grpc.MethodDescriptor.MethodType.UNARY)
  public static io.grpc.MethodDescriptor<omniharness.v1.SummarizeRequest,
      omniharness.v1.SummarizeResponse> getSummarizeMethod() {
    io.grpc.MethodDescriptor<omniharness.v1.SummarizeRequest, omniharness.v1.SummarizeResponse> getSummarizeMethod;
    if ((getSummarizeMethod = MemoryServiceGrpc.getSummarizeMethod) == null) {
      synchronized (MemoryServiceGrpc.class) {
        if ((getSummarizeMethod = MemoryServiceGrpc.getSummarizeMethod) == null) {
          MemoryServiceGrpc.getSummarizeMethod = getSummarizeMethod =
              io.grpc.MethodDescriptor.<omniharness.v1.SummarizeRequest, omniharness.v1.SummarizeResponse>newBuilder()
              .setType(io.grpc.MethodDescriptor.MethodType.UNARY)
              .setFullMethodName(generateFullMethodName(SERVICE_NAME, "Summarize"))
              .setSampledToLocalTracing(true)
              .setRequestMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  omniharness.v1.SummarizeRequest.getDefaultInstance()))
              .setResponseMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  omniharness.v1.SummarizeResponse.getDefaultInstance()))
              .setSchemaDescriptor(new MemoryServiceMethodDescriptorSupplier("Summarize"))
              .build();
        }
      }
    }
    return getSummarizeMethod;
  }

  /**
   * Creates a new async stub that supports all call types for the service
   */
  public static MemoryServiceStub newStub(io.grpc.Channel channel) {
    io.grpc.stub.AbstractStub.StubFactory<MemoryServiceStub> factory =
      new io.grpc.stub.AbstractStub.StubFactory<MemoryServiceStub>() {
        @java.lang.Override
        public MemoryServiceStub newStub(io.grpc.Channel channel, io.grpc.CallOptions callOptions) {
          return new MemoryServiceStub(channel, callOptions);
        }
      };
    return MemoryServiceStub.newStub(factory, channel);
  }

  /**
   * Creates a new blocking-style stub that supports unary and streaming output calls on the service
   */
  public static MemoryServiceBlockingStub newBlockingStub(
      io.grpc.Channel channel) {
    io.grpc.stub.AbstractStub.StubFactory<MemoryServiceBlockingStub> factory =
      new io.grpc.stub.AbstractStub.StubFactory<MemoryServiceBlockingStub>() {
        @java.lang.Override
        public MemoryServiceBlockingStub newStub(io.grpc.Channel channel, io.grpc.CallOptions callOptions) {
          return new MemoryServiceBlockingStub(channel, callOptions);
        }
      };
    return MemoryServiceBlockingStub.newStub(factory, channel);
  }

  /**
   * Creates a new ListenableFuture-style stub that supports unary calls on the service
   */
  public static MemoryServiceFutureStub newFutureStub(
      io.grpc.Channel channel) {
    io.grpc.stub.AbstractStub.StubFactory<MemoryServiceFutureStub> factory =
      new io.grpc.stub.AbstractStub.StubFactory<MemoryServiceFutureStub>() {
        @java.lang.Override
        public MemoryServiceFutureStub newStub(io.grpc.Channel channel, io.grpc.CallOptions callOptions) {
          return new MemoryServiceFutureStub(channel, callOptions);
        }
      };
    return MemoryServiceFutureStub.newStub(factory, channel);
  }

  /**
   */
  public interface AsyncService {

    /**
     */
    default void store(omniharness.v1.StoreRequest request,
        io.grpc.stub.StreamObserver<omniharness.v1.StoreResponse> responseObserver) {
      io.grpc.stub.ServerCalls.asyncUnimplementedUnaryCall(getStoreMethod(), responseObserver);
    }

    /**
     */
    default void retrieve(omniharness.v1.RetrieveRequest request,
        io.grpc.stub.StreamObserver<omniharness.v1.RetrieveResponse> responseObserver) {
      io.grpc.stub.ServerCalls.asyncUnimplementedUnaryCall(getRetrieveMethod(), responseObserver);
    }

    /**
     */
    default void searchSemantic(omniharness.v1.SemanticSearchRequest request,
        io.grpc.stub.StreamObserver<omniharness.v1.SemanticSearchResponse> responseObserver) {
      io.grpc.stub.ServerCalls.asyncUnimplementedUnaryCall(getSearchSemanticMethod(), responseObserver);
    }

    /**
     */
    default void delete(omniharness.v1.DeleteRequest request,
        io.grpc.stub.StreamObserver<omniharness.v1.DeleteResponse> responseObserver) {
      io.grpc.stub.ServerCalls.asyncUnimplementedUnaryCall(getDeleteMethod(), responseObserver);
    }

    /**
     */
    default void listCollections(omniharness.v1.ListCollRequest request,
        io.grpc.stub.StreamObserver<omniharness.v1.ListCollResponse> responseObserver) {
      io.grpc.stub.ServerCalls.asyncUnimplementedUnaryCall(getListCollectionsMethod(), responseObserver);
    }

    /**
     */
    default void summarize(omniharness.v1.SummarizeRequest request,
        io.grpc.stub.StreamObserver<omniharness.v1.SummarizeResponse> responseObserver) {
      io.grpc.stub.ServerCalls.asyncUnimplementedUnaryCall(getSummarizeMethod(), responseObserver);
    }
  }

  /**
   * Base class for the server implementation of the service MemoryService.
   */
  public static abstract class MemoryServiceImplBase
      implements io.grpc.BindableService, AsyncService {

    @java.lang.Override public final io.grpc.ServerServiceDefinition bindService() {
      return MemoryServiceGrpc.bindService(this);
    }
  }

  /**
   * A stub to allow clients to do asynchronous rpc calls to service MemoryService.
   */
  public static final class MemoryServiceStub
      extends io.grpc.stub.AbstractAsyncStub<MemoryServiceStub> {
    private MemoryServiceStub(
        io.grpc.Channel channel, io.grpc.CallOptions callOptions) {
      super(channel, callOptions);
    }

    @java.lang.Override
    protected MemoryServiceStub build(
        io.grpc.Channel channel, io.grpc.CallOptions callOptions) {
      return new MemoryServiceStub(channel, callOptions);
    }

    /**
     */
    public void store(omniharness.v1.StoreRequest request,
        io.grpc.stub.StreamObserver<omniharness.v1.StoreResponse> responseObserver) {
      io.grpc.stub.ClientCalls.asyncUnaryCall(
          getChannel().newCall(getStoreMethod(), getCallOptions()), request, responseObserver);
    }

    /**
     */
    public void retrieve(omniharness.v1.RetrieveRequest request,
        io.grpc.stub.StreamObserver<omniharness.v1.RetrieveResponse> responseObserver) {
      io.grpc.stub.ClientCalls.asyncUnaryCall(
          getChannel().newCall(getRetrieveMethod(), getCallOptions()), request, responseObserver);
    }

    /**
     */
    public void searchSemantic(omniharness.v1.SemanticSearchRequest request,
        io.grpc.stub.StreamObserver<omniharness.v1.SemanticSearchResponse> responseObserver) {
      io.grpc.stub.ClientCalls.asyncUnaryCall(
          getChannel().newCall(getSearchSemanticMethod(), getCallOptions()), request, responseObserver);
    }

    /**
     */
    public void delete(omniharness.v1.DeleteRequest request,
        io.grpc.stub.StreamObserver<omniharness.v1.DeleteResponse> responseObserver) {
      io.grpc.stub.ClientCalls.asyncUnaryCall(
          getChannel().newCall(getDeleteMethod(), getCallOptions()), request, responseObserver);
    }

    /**
     */
    public void listCollections(omniharness.v1.ListCollRequest request,
        io.grpc.stub.StreamObserver<omniharness.v1.ListCollResponse> responseObserver) {
      io.grpc.stub.ClientCalls.asyncUnaryCall(
          getChannel().newCall(getListCollectionsMethod(), getCallOptions()), request, responseObserver);
    }

    /**
     */
    public void summarize(omniharness.v1.SummarizeRequest request,
        io.grpc.stub.StreamObserver<omniharness.v1.SummarizeResponse> responseObserver) {
      io.grpc.stub.ClientCalls.asyncUnaryCall(
          getChannel().newCall(getSummarizeMethod(), getCallOptions()), request, responseObserver);
    }
  }

  /**
   * A stub to allow clients to do synchronous rpc calls to service MemoryService.
   */
  public static final class MemoryServiceBlockingStub
      extends io.grpc.stub.AbstractBlockingStub<MemoryServiceBlockingStub> {
    private MemoryServiceBlockingStub(
        io.grpc.Channel channel, io.grpc.CallOptions callOptions) {
      super(channel, callOptions);
    }

    @java.lang.Override
    protected MemoryServiceBlockingStub build(
        io.grpc.Channel channel, io.grpc.CallOptions callOptions) {
      return new MemoryServiceBlockingStub(channel, callOptions);
    }

    /**
     */
    public omniharness.v1.StoreResponse store(omniharness.v1.StoreRequest request) {
      return io.grpc.stub.ClientCalls.blockingUnaryCall(
          getChannel(), getStoreMethod(), getCallOptions(), request);
    }

    /**
     */
    public omniharness.v1.RetrieveResponse retrieve(omniharness.v1.RetrieveRequest request) {
      return io.grpc.stub.ClientCalls.blockingUnaryCall(
          getChannel(), getRetrieveMethod(), getCallOptions(), request);
    }

    /**
     */
    public omniharness.v1.SemanticSearchResponse searchSemantic(omniharness.v1.SemanticSearchRequest request) {
      return io.grpc.stub.ClientCalls.blockingUnaryCall(
          getChannel(), getSearchSemanticMethod(), getCallOptions(), request);
    }

    /**
     */
    public omniharness.v1.DeleteResponse delete(omniharness.v1.DeleteRequest request) {
      return io.grpc.stub.ClientCalls.blockingUnaryCall(
          getChannel(), getDeleteMethod(), getCallOptions(), request);
    }

    /**
     */
    public omniharness.v1.ListCollResponse listCollections(omniharness.v1.ListCollRequest request) {
      return io.grpc.stub.ClientCalls.blockingUnaryCall(
          getChannel(), getListCollectionsMethod(), getCallOptions(), request);
    }

    /**
     */
    public omniharness.v1.SummarizeResponse summarize(omniharness.v1.SummarizeRequest request) {
      return io.grpc.stub.ClientCalls.blockingUnaryCall(
          getChannel(), getSummarizeMethod(), getCallOptions(), request);
    }
  }

  /**
   * A stub to allow clients to do ListenableFuture-style rpc calls to service MemoryService.
   */
  public static final class MemoryServiceFutureStub
      extends io.grpc.stub.AbstractFutureStub<MemoryServiceFutureStub> {
    private MemoryServiceFutureStub(
        io.grpc.Channel channel, io.grpc.CallOptions callOptions) {
      super(channel, callOptions);
    }

    @java.lang.Override
    protected MemoryServiceFutureStub build(
        io.grpc.Channel channel, io.grpc.CallOptions callOptions) {
      return new MemoryServiceFutureStub(channel, callOptions);
    }

    /**
     */
    public com.google.common.util.concurrent.ListenableFuture<omniharness.v1.StoreResponse> store(
        omniharness.v1.StoreRequest request) {
      return io.grpc.stub.ClientCalls.futureUnaryCall(
          getChannel().newCall(getStoreMethod(), getCallOptions()), request);
    }

    /**
     */
    public com.google.common.util.concurrent.ListenableFuture<omniharness.v1.RetrieveResponse> retrieve(
        omniharness.v1.RetrieveRequest request) {
      return io.grpc.stub.ClientCalls.futureUnaryCall(
          getChannel().newCall(getRetrieveMethod(), getCallOptions()), request);
    }

    /**
     */
    public com.google.common.util.concurrent.ListenableFuture<omniharness.v1.SemanticSearchResponse> searchSemantic(
        omniharness.v1.SemanticSearchRequest request) {
      return io.grpc.stub.ClientCalls.futureUnaryCall(
          getChannel().newCall(getSearchSemanticMethod(), getCallOptions()), request);
    }

    /**
     */
    public com.google.common.util.concurrent.ListenableFuture<omniharness.v1.DeleteResponse> delete(
        omniharness.v1.DeleteRequest request) {
      return io.grpc.stub.ClientCalls.futureUnaryCall(
          getChannel().newCall(getDeleteMethod(), getCallOptions()), request);
    }

    /**
     */
    public com.google.common.util.concurrent.ListenableFuture<omniharness.v1.ListCollResponse> listCollections(
        omniharness.v1.ListCollRequest request) {
      return io.grpc.stub.ClientCalls.futureUnaryCall(
          getChannel().newCall(getListCollectionsMethod(), getCallOptions()), request);
    }

    /**
     */
    public com.google.common.util.concurrent.ListenableFuture<omniharness.v1.SummarizeResponse> summarize(
        omniharness.v1.SummarizeRequest request) {
      return io.grpc.stub.ClientCalls.futureUnaryCall(
          getChannel().newCall(getSummarizeMethod(), getCallOptions()), request);
    }
  }

  private static final int METHODID_STORE = 0;
  private static final int METHODID_RETRIEVE = 1;
  private static final int METHODID_SEARCH_SEMANTIC = 2;
  private static final int METHODID_DELETE = 3;
  private static final int METHODID_LIST_COLLECTIONS = 4;
  private static final int METHODID_SUMMARIZE = 5;

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
        case METHODID_STORE:
          serviceImpl.store((omniharness.v1.StoreRequest) request,
              (io.grpc.stub.StreamObserver<omniharness.v1.StoreResponse>) responseObserver);
          break;
        case METHODID_RETRIEVE:
          serviceImpl.retrieve((omniharness.v1.RetrieveRequest) request,
              (io.grpc.stub.StreamObserver<omniharness.v1.RetrieveResponse>) responseObserver);
          break;
        case METHODID_SEARCH_SEMANTIC:
          serviceImpl.searchSemantic((omniharness.v1.SemanticSearchRequest) request,
              (io.grpc.stub.StreamObserver<omniharness.v1.SemanticSearchResponse>) responseObserver);
          break;
        case METHODID_DELETE:
          serviceImpl.delete((omniharness.v1.DeleteRequest) request,
              (io.grpc.stub.StreamObserver<omniharness.v1.DeleteResponse>) responseObserver);
          break;
        case METHODID_LIST_COLLECTIONS:
          serviceImpl.listCollections((omniharness.v1.ListCollRequest) request,
              (io.grpc.stub.StreamObserver<omniharness.v1.ListCollResponse>) responseObserver);
          break;
        case METHODID_SUMMARIZE:
          serviceImpl.summarize((omniharness.v1.SummarizeRequest) request,
              (io.grpc.stub.StreamObserver<omniharness.v1.SummarizeResponse>) responseObserver);
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
          getStoreMethod(),
          io.grpc.stub.ServerCalls.asyncUnaryCall(
            new MethodHandlers<
              omniharness.v1.StoreRequest,
              omniharness.v1.StoreResponse>(
                service, METHODID_STORE)))
        .addMethod(
          getRetrieveMethod(),
          io.grpc.stub.ServerCalls.asyncUnaryCall(
            new MethodHandlers<
              omniharness.v1.RetrieveRequest,
              omniharness.v1.RetrieveResponse>(
                service, METHODID_RETRIEVE)))
        .addMethod(
          getSearchSemanticMethod(),
          io.grpc.stub.ServerCalls.asyncUnaryCall(
            new MethodHandlers<
              omniharness.v1.SemanticSearchRequest,
              omniharness.v1.SemanticSearchResponse>(
                service, METHODID_SEARCH_SEMANTIC)))
        .addMethod(
          getDeleteMethod(),
          io.grpc.stub.ServerCalls.asyncUnaryCall(
            new MethodHandlers<
              omniharness.v1.DeleteRequest,
              omniharness.v1.DeleteResponse>(
                service, METHODID_DELETE)))
        .addMethod(
          getListCollectionsMethod(),
          io.grpc.stub.ServerCalls.asyncUnaryCall(
            new MethodHandlers<
              omniharness.v1.ListCollRequest,
              omniharness.v1.ListCollResponse>(
                service, METHODID_LIST_COLLECTIONS)))
        .addMethod(
          getSummarizeMethod(),
          io.grpc.stub.ServerCalls.asyncUnaryCall(
            new MethodHandlers<
              omniharness.v1.SummarizeRequest,
              omniharness.v1.SummarizeResponse>(
                service, METHODID_SUMMARIZE)))
        .build();
  }

  private static abstract class MemoryServiceBaseDescriptorSupplier
      implements io.grpc.protobuf.ProtoFileDescriptorSupplier, io.grpc.protobuf.ProtoServiceDescriptorSupplier {
    MemoryServiceBaseDescriptorSupplier() {}

    @java.lang.Override
    public com.google.protobuf.Descriptors.FileDescriptor getFileDescriptor() {
      return omniharness.v1.Omniharness.getDescriptor();
    }

    @java.lang.Override
    public com.google.protobuf.Descriptors.ServiceDescriptor getServiceDescriptor() {
      return getFileDescriptor().findServiceByName("MemoryService");
    }
  }

  private static final class MemoryServiceFileDescriptorSupplier
      extends MemoryServiceBaseDescriptorSupplier {
    MemoryServiceFileDescriptorSupplier() {}
  }

  private static final class MemoryServiceMethodDescriptorSupplier
      extends MemoryServiceBaseDescriptorSupplier
      implements io.grpc.protobuf.ProtoMethodDescriptorSupplier {
    private final java.lang.String methodName;

    MemoryServiceMethodDescriptorSupplier(java.lang.String methodName) {
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
      synchronized (MemoryServiceGrpc.class) {
        result = serviceDescriptor;
        if (result == null) {
          serviceDescriptor = result = io.grpc.ServiceDescriptor.newBuilder(SERVICE_NAME)
              .setSchemaDescriptor(new MemoryServiceFileDescriptorSupplier())
              .addMethod(getStoreMethod())
              .addMethod(getRetrieveMethod())
              .addMethod(getSearchSemanticMethod())
              .addMethod(getDeleteMethod())
              .addMethod(getListCollectionsMethod())
              .addMethod(getSummarizeMethod())
              .build();
        }
      }
    }
    return result;
  }
}
