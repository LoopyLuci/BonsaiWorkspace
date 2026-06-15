import asyncio
import json
import os
import websockets
from pathlib import Path

async def call(ws, method, params, req_id):
    msg = {"jsonrpc": "2.0", "id": req_id, "method": method, "params": params}
    await ws.send(json.dumps(msg))
    resp = await ws.recv()
    return json.loads(resp)

async def main():
    appdata = Path(os.getenv("APPDATA"))
    base = appdata / "bonsai"

    port_file = base / "daemon_port"
    token_file = base / "daemon_token"
    if not port_file.exists():
        port_file = base / "vscode_port"
    if not token_file.exists():
        token_file = base / "vscode_token"

    port = port_file.read_text().strip()
    token = token_file.read_text().strip()
    uri = f"ws://127.0.0.1:{port}/ws"

    async with websockets.connect(uri) as ws:
        # Authenticate (server handles this separately)
        auth_resp = await call(ws, "auth", {"token": token}, 0)
        print("1. AUTH:", auth_resp)

        # 2. Identity create (generate if phrase omitted)
        id_resp = await call(ws, "identity.create", {
            # omit phrase to let server generate a fresh identity
            "passphrase": "testpassword123"
        }, 1)
        print("2. IDENTITY CREATE:", id_resp)

        # 3. Identity get
        get_resp = await call(ws, "identity.get", {}, 2)
        print("3. IDENTITY GET:", get_resp)

        # 4. Contacts list (should be empty)
        list_resp = await call(ws, "contacts.list", {}, 3)
        print("4. CONTACTS LIST:", list_resp)

        # 5. Add contact (server expects a `contact` object)
        add_resp = await call(ws, "contacts.add_contact", {
            "contact": {
                "public_key": "0x0000000000000000000000000000000000000000000000000000000000000000",
                "display_name": "Test Peer"
            }
        }, 4)
        print("5. ADD CONTACT:", add_resp)

        # 6. Contacts list (should include added contact)
        list2_resp = await call(ws, "contacts.list", {}, 5)
        print("6. CONTACTS LIST (after add):", list2_resp)

        # 6.5 Generate a simple UI manifest
        gen_resp = await call(ws, "ui.generate", {"description": "Generate a simple transfer dashboard with three cards and a natural language input."}, 100)
        print("6.5 UI GENERATE:", gen_resp)

        # 6.6 Publish the manifest to CAS
        gen_manifest = gen_resp.get("result", {})
        pub_resp = await call(ws, "ui.publish", {"manifest": gen_manifest}, 101)
        print("6.6 UI PUBLISH:", pub_resp)

        # 6.7 Hot-reload the published manifest
        hot_resp = await call(ws, "ui.hot_reload", {"manifest": gen_manifest}, 102)
        print("6.7 UI HOT RELOAD:", hot_resp)

        # 6.8 Rollback (attempt)
        rollback_resp = await call(ws, "ui.rollback", {"id": gen_manifest.get("id")}, 103)
        print("6.8 UI ROLLBACK:", rollback_resp)

        # 7. Transfer: send file (loopback). Server expects `file_path` param.
        test_file = Path("test_transfer.txt")
        test_file.write_text("Hello, Bonsai transfer test!")

        send_resp = await call(ws, "transfer.send_file", {
            "file_path": str(test_file.absolute())
        }, 6)
        print("7. TRANSFER SEND:", send_resp)
        transfer_id = send_resp.get("result", {}).get("id", "")
        print(f"   Transfer ID: {transfer_id}")

        # 8. List transfers
        list_t_resp = await call(ws, "transfer.list", {}, 7)
        print("8. TRANSFERS LIST:", list_t_resp)

        # 9. Cancel transfer (if present)
        if transfer_id:
            cancel_resp = await call(ws, "transfer.cancel", {"id": transfer_id}, 8)
            print("9. TRANSFER CANCEL:", cancel_resp)

        # 10. APL evaluation (server accepts `src` or `code`)
        apl_resp = await call(ws, "data.eval_apl", {"src": "[2, 3, 5, 7, 11]"}, 9)
        print("10. APL EVAL:", apl_resp)

        # 11. Deno script (server expects `code`)
        deno_resp = await call(ws, "deno.run_script", {"code": "1 + 1"}, 10)
        print("11. DENO SCRIPT:", deno_resp)

        # 12. Lean verification (send source string). If test.lean exists, send file contents.
        lean_source = "theorem trivial : True := by trivial"
        lean_path = Path("test.lean")
        if lean_path.exists():
            lean_source = lean_path.read_text()
        lean_resp = await call(ws, "verify.check_lean", {"source": lean_source}, 11)
        print("12. LEAN VERIFY:", lean_resp)

        # Cleanup
        test_file.unlink(missing_ok=True)

if __name__ == '__main__':
    asyncio.run(main())
