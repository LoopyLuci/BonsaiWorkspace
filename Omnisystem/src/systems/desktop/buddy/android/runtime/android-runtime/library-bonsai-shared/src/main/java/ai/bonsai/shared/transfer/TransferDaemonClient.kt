package ai.bonsai.shared.transfer

import android.content.Context
import kotlinx.coroutines.suspendCancellableCoroutine
import kotlin.coroutines.resume
import kotlin.coroutines.resumeWithException

class TransferDaemonClient(private val context: Context) {
    private var nativePtr: Long = 0

    fun start(configPath: String): Boolean {
        nativePtr = nativeStart(configPath)
        return nativePtr != 0L
    }

    suspend fun connectToPeer(peerId: String, token: ByteArray): Session = suspendCancellableCoroutine { cont ->
        nativeConnect(peerId, token, object : ConnectionCallback {
            override fun onSuccess(sessionPtr: Long) {
                cont.resume(Session(sessionPtr, this@TransferDaemonClient))
            }
            override fun onError(err: String) {
                cont.resumeWithException(Exception(err))
            }
        })
    }

    fun openStream(session: Session, streamType: String): Stream {
        val streamPtr = nativeOpenStream(session.ptr, streamType)
        return Stream(streamPtr, session, this)
    }

    fun sendOnStream(stream: Stream, data: ByteArray) {
        nativeSend(stream.ptr, data)
    }

    fun stop() {
        if (nativePtr != 0L) {
            nativeStop(nativePtr)
            nativePtr = 0L
        }
    }

    private external fun nativeStart(configPath: String): Long
    private external fun nativeConnect(peerId: String, token: ByteArray, callback: ConnectionCallback)
    private external fun nativeOpenStream(sessionPtr: Long, type: String): Long
    private external fun nativeSend(streamPtr: Long, data: ByteArray)
    private external fun nativeStop(ptr: Long)

    interface ConnectionCallback {
        fun onSuccess(sessionPtr: Long)
        fun onError(err: String)
    }

    inner class Session(val ptr: Long, val client: TransferDaemonClient) {
        fun close() { nativeCloseSession(ptr) }
        private external fun nativeCloseSession(ptr: Long)
    }

    inner class Stream(val ptr: Long, val session: Session, val client: TransferDaemonClient) {
        fun close() { nativeCloseStream(ptr) }
        private external fun nativeCloseStream(ptr: Long)
    }
}
