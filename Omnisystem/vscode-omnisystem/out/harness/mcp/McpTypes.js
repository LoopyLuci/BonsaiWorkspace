"use strict";
// MCP (Model Context Protocol) shared types.
// Protocol: JSON-RPC 2.0. Spec: https://modelcontextprotocol.io
// Transports supported: stdio (spawned subprocess) and Streamable HTTP.
Object.defineProperty(exports, "__esModule", { value: true });
exports.MCP_PREFIX = exports.MCP_PROTOCOL_VERSION = void 0;
exports.qualifyToolName = qualifyToolName;
exports.parseQualifiedName = parseQualifiedName;
exports.MCP_PROTOCOL_VERSION = '2024-11-05';
/** The MCP tool-call namespace prefix used to route to an MCP server. */
exports.MCP_PREFIX = 'mcp__';
function qualifyToolName(serverId, tool) {
    return `${exports.MCP_PREFIX}${serverId}__${tool}`;
}
/** Parse `mcp__<serverId>__<tool>` → { serverId, tool } (tool may contain `__`? no). */
function parseQualifiedName(qualified) {
    if (!qualified.startsWith(exports.MCP_PREFIX)) {
        return undefined;
    }
    const rest = qualified.slice(exports.MCP_PREFIX.length);
    const sep = rest.indexOf('__');
    if (sep === -1) {
        return undefined;
    }
    return { serverId: rest.slice(0, sep), tool: rest.slice(sep + 2) };
}
//# sourceMappingURL=McpTypes.js.map