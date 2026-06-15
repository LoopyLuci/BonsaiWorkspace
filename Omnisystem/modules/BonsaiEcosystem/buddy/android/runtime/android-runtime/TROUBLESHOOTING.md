# Troubleshooting Guide

Common issues and solutions for Bonsai Buddy's remote desktop client.

## Connection Issues

### "Connection failed: Connection refused"

**Causes:**
- Remote desktop is offline or not running TransferDaemon
- Firewall blocking connection
- Wrong peer ID
- Network unreachable

**Solutions:**
1. Verify remote desktop is online
   - Check system notifications on remote machine
   - Try connecting from another device
2. Check firewall settings
   - Windows: Allow Bonsai in Windows Defender Firewall
   - Linux: Check iptables or ufw rules
   - macOS: Check System Preferences → Security & Privacy
3. Verify peer ID is correct
   - Re-scan for devices in Connection Manager
   - Check peer name matches in list
4. Check network connectivity
   - Ping remote host from phone
   - Check Wi-Fi or Ethernet connection
   - Try mobile hotspot (if available)

### "Connection timeout after 30 seconds"

**Causes:**
- Network latency too high
- Firewall dropping packets
- TransferDaemon not responding

**Solutions:**
1. Check network latency
   - Connect to 5GHz Wi-Fi (if available)
   - Move closer to router
   - Reduce network load (pause downloads, disable other connections)
2. Increase timeout (advanced)
   - Settings → Developer Options → Connection Timeout
   - Default: 30 seconds, can increase to 60 seconds
3. Try different connection method
   - If on Wi-Fi, try mobile hotspot
   - If on mobile, try Wi-Fi

### "Lost connection: Connection reset by peer"

**Causes:**
- Remote desktop crashed or disconnected
- Network interruption
- Session timeout (>30 minutes idle)

**Solutions:**
1. Reconnect to remote desktop
   - Tap "Disconnect"
   - Tap device in list to reconnect
2. Check remote machine
   - Verify it's still running
   - Check TransferDaemon logs
3. Wait and retry
   - Session may be recovering
   - Network may be temporarily down
   - Try again in 30 seconds

## Video Playback Issues

### "Black screen with no video"

**Causes:**
- MediaCodec decoder not initialized
- H.264/H.265 codec not supported
- Video stream not sending data

**Solutions:**
1. Verify codec support
   - Check device specs (Redmi Note 12 Pro supports both)
   - In advanced settings, check "Supported Codecs"
2. Restart connection
   - Disconnect and reconnect
   - Check connection bar at top
3. Check remote desktop screen
   - Verify remote desktop is displaying
   - Try moving mouse on remote — should respond
4. Check for hardware decoder errors
   - In Diagnostics (future), view MediaCodec errors
   - On some devices, try "CPU decode" mode

### "Video stuttering or frame drops"

**Causes:**
- Network bandwidth insufficient
- Decoder falling behind
- Phone CPU overload
- Resolution too high for device

**Solutions:**
1. **Lower desktop resolution**
   - Reduce from 1920x1080 to 1280x720
   - Lower refresh rate from 60 to 30 fps
   - Reduces bitrate by ~50%

2. **Improve network connectivity**
   - Move closer to Wi-Fi router
   - Switch from 2.4GHz to 5GHz band
   - Reduce network load (pause large transfers)
   - Connect via Ethernet (if available on desktop)

3. **Free up phone resources**
   - Close background apps
   - Stop music/video playback
   - Disable notifications temporarily
   - Enable Battery Saver mode (may limit FPS)

4. **Check connection statistics**
   - Watch FPS in connection bar
   - Should be >30 FPS
   - If <20 FPS, connection is bottleneck

### "Green screen or color distortion"

**Causes:**
- Codec mismatch
- Color format conversion error
- Corrupted frame data

**Solutions:**
1. Restart connection
   - Disconnect: Tap ✕ button
   - Reconnect: Select device again
2. Check codec
   - Some devices may default to wrong codec
   - Try switching H.264 ↔ H.265 in advanced settings
3. Lower bitrate
   - Forces lower quality but more stable decoding
   - May reduce color errors

### "Audio not working"

**Status:** Audio support coming in Phase 2
**Workaround:** Use remote desktop's built-in speakers or headphones

## Input Issues

### "Touch input not registering"

**Causes:**
- App doesn't have focus
- Touch coordinates out of bounds
- Input stream disconnected

**Solutions:**
1. Verify video is playing
   - Black screen means input won't work
   - Check connection bar shows "Connected"
2. Tap on video area
   - Ensure app has focus
   - Tap in center of screen first
3. Reconnect
   - Disconnect and reconnect
   - Reset input stream

### "Keyboard input not sending"

**Causes:**
- On-screen keyboard not visible
- Keyboard stream not connected
- Key mapping issue

**Solutions:**
1. Show on-screen keyboard
   - Tap ⌨ button in toolbar
   - Or use 3-finger swipe up
2. Check modifier keys
   - Verify Ctrl/Alt/Shift buttons show correctly
   - Tap a letter to verify it sends
3. Try physical keyboard
   - If Bluetooth keyboard available, use it
   - Faster and more reliable
4. Check keyboard language
   - Remote desktop keyboard layout must match
   - English (US) QWERTY is default

### "Gestures not working correctly"

#### Drag movements seem reversed
- **Solution:** Check mouse mode
- Tap button in toolbar to toggle Absolute ↔ Relative
- Absolute mode is usually better for touch

#### Scroll not moving page
- **Solution:** Try two-finger drag instead
- On some apps, may need to scroll on specific area
- Some web content may need vertical scroll

#### Double-tap opening wrong app
- **Solution:** Reduce double-tap timeout
- Settings → Gesture Sensitivity → Double-tap Time
- Default 300ms, try reducing to 200ms

## Display Issues

### "On-screen keyboard covers content"

**Solutions:**
1. Reposition keyboard
   - On some screens, keyboard appears at bottom
   - Swipe it to side if possible (future)
2. Hide keyboard when not needed
   - Tap ⌨ to hide
   - Tap ✕ button on keyboard
   - Use 3-finger swipe down
3. Use physical keyboard
   - Connect Bluetooth keyboard for permanent solution

### "Connection bar blocking important content"

**Solutions:**
1. Disable stats display
   - Settings → Display → Show Stats
   - Uncheck to hide (but lose diagnostics)
2. Rotate device
   - Landscape mode may give more space
   - Check if app supports rotation

### "Text appears blurry or pixelated"

**Causes:**
- Resolution mismatch
- Screen upscaling
- Font rendering at low DPI

**Solutions:**
1. Increase remote resolution
   - 1920x1080 or higher
   - Minimum 1280x720 for readable text
2. Zoom into remote desktop
   - Pinch gesture to zoom in
   - May need to navigate to see full screen
3. Use higher refresh rate
   - 60fps instead of 30fps
   - May improve sharpness perception

## Performance Issues

### "App crashes when connecting"

**Causes:**
- Insufficient memory
- Codec initialization failure
- Null pointer in input handling

**Solutions:**
1. Free up phone memory
   - Close other apps
   - Restart phone
   - Clear app cache: Settings → Apps → Bonsai Buddy → Storage → Clear Cache
2. Check device compatibility
   - Minimum API 26 (Android 8)
   - MediaCodec support required
   - 2GB RAM recommended, 4GB+ better
3. Try lower resolution
   - Start with 1280x720 instead of 1920x1080
   - Reduces memory usage significantly

### "High CPU usage / phone gets hot"

**Causes:**
- Software video decoding (no hardware acceleration)
- High frame rate / high resolution
- Multiple connections
- Background processes

**Solutions:**
1. Check hardware acceleration
   - Settings → Developer Options → Hardware Video Codec
   - Should be enabled (Redmi Note 12 Pro has excellent support)
2. Lower resolution/bitrate
   - Reduces CPU load by 50% per resolution step
   - 1280x720 uses ~20% CPU, 1920x1080 uses ~30%
3. Reduce frame rate
   - 30 fps instead of 60 fps
   - Cuts decoding CPU in half
4. Close background apps
   - Use task manager to force-stop unused apps
   - Disable auto-sync
5. Enable Battery Saver
   - Settings → Battery → Battery Saver
   - May reduce video quality but saves power

### "High battery drain"

**Causes:**
- High resolution/bitrate
- 60 fps decoding
- Screen always on
- CPU maxed out

**Solutions:**
1. Reduce resolution
   - 1280x720 @ 30fps uses ~30% less battery than 1920x1080 @ 60fps
2. Enable Battery Saver
   - May limit to 30 fps and lower brightness
3. Enable screen timeout
   - Screen off during idle connection
   - Settings → Display → Screen Timeout → 5 minutes
4. Use lower brightness
   - Drag brightness slider down
   - Biggest battery drain on AMOLED screens

## Network Issues

### "Packet loss detected (>5% in stats)"

**Causes:**
- Weak Wi-Fi signal
- Network congestion
- Distance from router

**Solutions:**
1. Move closer to router
   - Optimal distance: <20 feet
   - Through 1-2 walls at most
2. Switch to 5GHz Wi-Fi
   - Faster but shorter range
   - Usually less congestion
3. Reduce bitrate
   - Lower resolution reduces data
   - Fewer dropped frames = less retransmission
4. Check for interference
   - Microwave ovens
   - Cordless phones
   - Other 2.4GHz devices
   - Move away from these

### "Latency is high (>50ms in stats)"

**Causes:**
- Distance from server
- High network latency
- Congested network

**Solutions:**
1. Check internet speed
   - Use speed test app
   - Should have <50ms latency to server
2. Switch Wi-Fi networks
   - Try 5GHz if available
   - Try different router
3. Reduce network load
   - Pause downloads/uploads
   - Close video streaming apps
4. Use wired connection
   - If desktop has Ethernet, connect it there
   - Reduces latency on server side

## Pairing / Device Management

### "Device not showing in list"

**Causes:**
- Device not broadcasting mDNS
- Device offline
- Different network/subnet
- Firewall blocking mDNS

**Solutions:**
1. Verify device is online
   - Check on remote machine
   - Verify TransferDaemon is running
2. Check network
   - Phone and desktop on same Wi-Fi
   - Check IP addresses are in same subnet
3. Restart discovery
   - Tap "Rescan" in device list
   - Wait 10 seconds for mDNS to respond
4. Check firewall
   - Allow Bonsai mDNS traffic (port 5353)
   - Windows: Windows Defender Firewall
   - Linux: ufw or iptables
   - macOS: System Preferences → Security

### "QR code scanning not working"

**Causes:**
- Camera permission not granted
- Poor lighting
- QR code damaged
- Wrong QR code version

**Solutions:**
1. Grant camera permission
   - Settings → Apps → Bonsai Buddy → Permissions → Camera
2. Improve lighting
   - Move to brighter area
   - Avoid glare on QR code
3. Regenerate QR code
   - In Connection Manager, tap "Generate QR"
   - Try scanning again
4. Manual entry
   - If QR fails, manually enter peer ID
   - Connection Manager → + → Enter Peer ID

### "Token expired error"

**Causes:**
- Token older than 24 hours
- System clock out of sync
- Device pairing revoked on desktop

**Solutions:**
1. Regenerate token
   - Connection Manager → Device → Regenerate Token
   - Default valid for 24 hours
2. Sync system clock
   - Settings → Date & Time → Set Automatically
   - Should sync to NTP server
3. Check desktop
   - Verify pairing still exists
   - May need to re-pair device

## Advanced Diagnostics

### Enable Debug Logging

1. Go to Settings → Developer Options
2. Enable "Remote Desktop Debug Logging"
3. Logs saved to: `/sdcard/Android/data/ai.bonsai.buddy/files/brdf.log`
4. Share log file for support

### Check Codec Support

1. Settings → About → Device Codecs
2. Should show:
   - H.264 (avc) — SUPPORTED
   - H.265 (hevc) — SUPPORTED
3. If not showing, device may not have hardware decoders

### Monitor Real-time Stats

- Connection bar shows: FPS, Bitrate, Latency, Packet Loss
- Open connection bar details for extended stats:
  - Video bytes received
  - Decoded frames / Dropped frames
  - Input events sent
  - Session uptime

## Getting Help

If issue persists:

1. **Gather diagnostics**
   - Connection bar screenshot
   - Debug log (Settings → Developer → Export Log)
   - Device model and Android version
   - Last 5 lines of error message

2. **Report in GitHub Issues**
   - https://github.com/bonsai/bonsai-workspace/issues
   - Include diagnostics
   - Description of steps to reproduce

3. **Community Discord**
   - https://discord.gg/bonsai
   - Real-time help from developers
   - Community solutions often available

## FAQ

**Q: Why is there latency with a local network?**
A: Latency includes codec/decode overhead, OS scheduling, and frame buffering. ~10-20ms is typical even with local connection.

**Q: Can I control multiple desktops at once?**
A: Not currently. Future version will support parallel sessions.

**Q: What happens if I rotate the phone?**
A: Aspect ratio changes. Video will stretch unless app supports rotation (future feature).

**Q: Can I record the session?**
A: Not currently. Video recording coming in Phase 2.

**Q: Is it secure?**
A: Yes. Capability tokens + TransferDaemon encryption. No plaintext video over network.

**Q: Can I use this over the internet / VPN?**
A: Yes. Latency will be higher but should work. VPN overhead may reduce bitrate.
