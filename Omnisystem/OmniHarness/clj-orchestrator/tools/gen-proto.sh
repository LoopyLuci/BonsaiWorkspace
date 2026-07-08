#!/usr/bin/env bash
# Regenerate the Java protobuf + gRPC stubs consumed by omniharness.client.
#
# Requires:
#   - protoc            (protocol buffers compiler) — use the 25.x line so the
#                       emitted message classes match protobuf-java 3.25.x, the
#                       runtime pulled in transitively by grpc-protobuf 1.68.x.
#                       (protoc 28.x emits protobuf-java 4.x API and will NOT
#                        compile against the 3.25 runtime.)
#   - protoc-gen-grpc-java  (gRPC Java code generator plugin)
#
# Override the tool locations with the PROTOC / GRPC_JAVA_PLUGIN env vars.
# The generated sources land in gen-java/, which project.clj compiles via
# :java-source-paths.
set -euo pipefail

here="$(cd "$(dirname "$0")/.." && pwd)"
proto_dir="$here/../proto"
out_dir="$here/gen-java"

PROTOC="${PROTOC:-protoc}"
GRPC_JAVA_PLUGIN="${GRPC_JAVA_PLUGIN:-protoc-gen-grpc-java}"

mkdir -p "$out_dir"
echo "[gen-proto] protoc         = $PROTOC"
echo "[gen-proto] grpc-java plug = $GRPC_JAVA_PLUGIN"
echo "[gen-proto] proto          = $proto_dir/omniharness.proto"
echo "[gen-proto] out            = $out_dir"

"$PROTOC" \
  --plugin=protoc-gen-grpc-java="$GRPC_JAVA_PLUGIN" \
  --java_out="$out_dir" \
  --grpc-java_out="$out_dir" \
  -I "$proto_dir" \
  "$proto_dir/omniharness.proto"

echo "[gen-proto] done. Generated:"
find "$out_dir" -name '*.java' -printf '  %p\n'
