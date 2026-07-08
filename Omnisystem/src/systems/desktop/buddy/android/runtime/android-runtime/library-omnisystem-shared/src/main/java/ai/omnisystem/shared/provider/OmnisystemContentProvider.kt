package ai.omnisystem.shared.provider

import android.content.ContentProvider
import android.content.ContentValues
import android.content.UriMatcher
import android.database.Cursor
import android.net.Uri
import ai.omnisystem.shared.db.OmnisystemDatabase

class OmnisystemContentProvider : ContentProvider() {
    private val AUTHORITY = "ai.omnisystem.shared.provider"
    private lateinit var database: OmnisystemDatabase

    override fun onCreate(): Boolean {
        database = OmnisystemDatabase.getInstance(context!!)
        return true
    }

    override fun query(
        uri: Uri,
        projection: Array<out String>?,
        selection: String?,
        selectionArgs: Array<out String>?,
        sortOrder: String?
    ): Cursor? = null

    override fun getType(uri: Uri): String? = null
    override fun insert(uri: Uri, values: ContentValues?): Uri? = null
    override fun delete(uri: Uri, selection: String?, selectionArgs: Array<out String>?): Int = 0
    override fun update(uri: Uri, values: ContentValues?, selection: String?, selectionArgs: Array<out String>?): Int = 0
}
