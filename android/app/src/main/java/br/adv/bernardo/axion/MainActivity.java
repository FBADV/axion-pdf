package br.adv.bernardo.axion;

import android.annotation.SuppressLint;
import android.app.Activity;
import android.content.ClipData;
import android.content.Context;
import android.content.Intent;
import android.content.res.Configuration;
import android.database.Cursor;
import android.graphics.Color;
import android.net.Uri;
import android.os.Bundle;
import android.os.CancellationSignal;
import android.os.Handler;
import android.os.Looper;
import android.os.ParcelFileDescriptor;
import android.print.PageRange;
import android.print.PrintAttributes;
import android.print.PrintDocumentAdapter;
import android.print.PrintDocumentInfo;
import android.print.PrintManager;
import android.provider.OpenableColumns;
import android.util.Base64;
import android.webkit.JavascriptInterface;
import android.webkit.ValueCallback;
import android.webkit.WebChromeClient;
import android.webkit.WebResourceRequest;
import android.webkit.WebResourceResponse;
import android.webkit.WebSettings;
import android.webkit.WebView;
import android.webkit.WebViewClient;
import android.widget.Toast;

import androidx.core.content.FileProvider;
import androidx.webkit.WebViewAssetLoader;

import java.io.ByteArrayOutputStream;
import java.io.File;
import java.io.FileOutputStream;
import java.io.InputStream;
import java.io.OutputStream;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.List;

/**
 * Axion for Android: the app's WebAssembly build, served from the APK's assets at a secure
 * https origin, in a full-screen WebView. The injected bridge (assets/axion-bridge.js) turns the
 * web build's downloads into the system "Save as" dialog, its print requests into Android printing,
 * and files shared with or opened in Axion into documents in the app.
 */
public class MainActivity extends Activity {
    private static final String ORIGIN = "https://appassets.androidplatform.net";
    private static final String START = ORIGIN + "/assets/web/index.html";
    private static final int PICK = 1;
    private static final int SAVE = 2;

    private WebView web;
    private ValueCallback<Uri[]> pickCallback;
    private byte[] pendingSave;
    private boolean ready;
    private final List<Uri> pendingOpen = new ArrayList<>();
    private final Handler main = new Handler(Looper.getMainLooper());

    @SuppressLint({"SetJavaScriptEnabled", "AddJavascriptInterface"})
    @Override
    protected void onCreate(Bundle state) {
        super.onCreate(state);
        web = new WebView(this);
        web.setBackgroundColor(isNight() ? Color.rgb(0x0E, 0x14, 0x20) : Color.rgb(0xED, 0xF2, 0xF9));
        setContentView(web);
        applyInsets();

        WebSettings s = web.getSettings();
        s.setJavaScriptEnabled(true);
        s.setDomStorageEnabled(true);
        s.setDatabaseEnabled(true);
        s.setAllowFileAccess(false);
        s.setAllowContentAccess(true);
        s.setMediaPlaybackRequiresUserGesture(true);
        s.setSupportZoom(false);
        s.setBuiltInZoomControls(false);
        s.setTextZoom(100);

        final WebViewAssetLoader loader = new WebViewAssetLoader.Builder()
                .addPathHandler("/assets/", new WebViewAssetLoader.AssetsPathHandler(this))
                .build();
        web.setWebViewClient(new WebViewClient() {
            @Override
            public WebResourceResponse shouldInterceptRequest(WebView view, WebResourceRequest request) {
                WebResourceResponse r = loader.shouldInterceptRequest(request.getUrl());
                if (r != null && request.getUrl().getPath() != null && request.getUrl().getPath().endsWith(".wasm")) {
                    r.setMimeType("application/wasm");
                }
                return r;
            }

            @Override
            public boolean shouldOverrideUrlLoading(WebView view, WebResourceRequest request) {
                Uri u = request.getUrl();
                if (u.toString().startsWith(ORIGIN)) return false;
                // Links in documents and Help open in the browser (or mail app).
                try {
                    startActivity(new Intent(Intent.ACTION_VIEW, u));
                } catch (Exception e) {
                    toast("Não foi possível abrir o link.");
                }
                return true;
            }

            @Override
            public void onPageFinished(WebView view, String url) {
                injectBridge();
            }
        });
        web.setWebChromeClient(new WebChromeClient() {
            @Override
            public boolean onShowFileChooser(WebView view, ValueCallback<Uri[]> callback, FileChooserParams params) {
                if (pickCallback != null) pickCallback.onReceiveValue(null);
                pickCallback = callback;
                Intent i = new Intent(Intent.ACTION_OPEN_DOCUMENT);
                i.addCategory(Intent.CATEGORY_OPENABLE);
                i.setType("*/*");
                // The web app's own accept list (".pdf", "image/png"…), as MIME types.
                List<String> types = new ArrayList<>();
                boolean unknown = false;
                for (String t : params.getAcceptTypes()) {
                    if (t == null) continue;
                    for (String one : t.split(",")) {
                        String m = one.trim().toLowerCase(java.util.Locale.ROOT);
                        if (m.startsWith(".")) m = android.webkit.MimeTypeMap.getSingleton().getMimeTypeFromExtension(m.substring(1));
                        if (m == null || !m.contains("/")) {
                            if (!one.trim().isEmpty()) unknown = true;
                        } else if (!types.contains(m)) {
                            types.add(m);
                        }
                    }
                }
                // A type Android doesn't know (.md on older phones) would hide those files: show all.
                if (!types.isEmpty() && !unknown) i.putExtra(Intent.EXTRA_MIME_TYPES, types.toArray(new String[0]));
                i.putExtra(Intent.EXTRA_ALLOW_MULTIPLE, params.getMode() == FileChooserParams.MODE_OPEN_MULTIPLE);
                try {
                    startActivityForResult(i, PICK);
                } catch (Exception e) {
                    pickCallback = null;
                    return false;
                }
                return true;
            }
        });
        web.addJavascriptInterface(new Bridge(), "AxionAndroid");
        collectIncoming(getIntent());
        web.loadUrl(START);
    }

    private boolean isNight() {
        return (getResources().getConfiguration().uiMode & Configuration.UI_MODE_NIGHT_MASK) == Configuration.UI_MODE_NIGHT_YES;
    }

    /** Keep the app clear of the status bar, the navigation bar and the keyboard. */
    private void applyInsets() {
        web.setOnApplyWindowInsetsListener((v, insets) -> {
            int top = insets.getSystemWindowInsetTop();
            int bottom = insets.getSystemWindowInsetBottom();
            int left = insets.getSystemWindowInsetLeft();
            int right = insets.getSystemWindowInsetRight();
            v.setPadding(left, top, right, bottom);
            return insets;
        });
    }

    private void injectBridge() {
        String js = readAsset("axion-bridge.js");
        if (js == null) return;
        web.evaluateJavascript(js, value -> {
            ready = true;
            flushOpen();
        });
    }

    private String readAsset(String name) {
        try (InputStream in = getAssets().open(name)) {
            return new String(readAll(in), StandardCharsets.UTF_8);
        } catch (Exception e) {
            return null;
        }
    }

    private static byte[] readAll(InputStream in) throws java.io.IOException {
        ByteArrayOutputStream out = new ByteArrayOutputStream();
        byte[] buf = new byte[64 * 1024];
        int n;
        while ((n = in.read(buf)) > 0) out.write(buf, 0, n);
        return out.toByteArray();
    }

    // ---- files opened in or shared with Axion ----

    @Override
    protected void onNewIntent(Intent intent) {
        super.onNewIntent(intent);
        collectIncoming(intent);
        flushOpen();
    }

    private void collectIncoming(Intent intent) {
        if (intent == null) return;
        String action = intent.getAction();
        if (Intent.ACTION_VIEW.equals(action) && intent.getData() != null) {
            pendingOpen.add(intent.getData());
        } else if (Intent.ACTION_SEND.equals(action)) {
            Uri u = intent.getParcelableExtra(Intent.EXTRA_STREAM);
            if (u != null) pendingOpen.add(u);
        } else if (Intent.ACTION_SEND_MULTIPLE.equals(action)) {
            ArrayList<Uri> list = intent.getParcelableArrayListExtra(Intent.EXTRA_STREAM);
            if (list != null) pendingOpen.addAll(list);
        }
    }

    private void flushOpen() {
        if (!ready || pendingOpen.isEmpty()) return;
        // The web app needs a moment after loading before it accepts dropped files.
        final List<Uri> uris = new ArrayList<>(pendingOpen);
        pendingOpen.clear();
        main.postDelayed(() -> {
            for (Uri u : uris) openInApp(u);
        }, 1500);
    }

    private void openInApp(Uri u) {
        try (InputStream in = getContentResolver().openInputStream(u)) {
            if (in == null) return;
            byte[] data = readAll(in);
            String name = displayName(u);
            String b64 = Base64.encodeToString(data, Base64.NO_WRAP);
            String call = "window.axionOpenFile && window.axionOpenFile(" + jsString(name) + "," + jsString(b64) + ");";
            web.evaluateJavascript(call, null);
        } catch (Exception e) {
            toast("Não foi possível abrir o arquivo.");
        }
    }

    private String displayName(Uri u) {
        try (Cursor c = getContentResolver().query(u, new String[]{OpenableColumns.DISPLAY_NAME}, null, null, null)) {
            if (c != null && c.moveToFirst()) {
                String n = c.getString(0);
                if (n != null && !n.isEmpty()) return n;
            }
        } catch (Exception ignored) {
        }
        String last = u.getLastPathSegment();
        return last != null ? last : "documento";
    }

    private static String jsString(String s) {
        StringBuilder b = new StringBuilder("\"");
        for (char c : s.toCharArray()) {
            switch (c) {
                case '"': b.append("\\\""); break;
                case '\\': b.append("\\\\"); break;
                case '\n': b.append("\\n"); break;
                case '\r': b.append("\\r"); break;
                case '<': b.append("\\u003c"); break;
                case ' ': b.append("\\u2028"); break;
                case ' ': b.append("\\u2029"); break;
                default: b.append(c);
            }
        }
        return b.append('"').toString();
    }

    // ---- save, share and print, called from axion-bridge.js ----

    private class Bridge {
        @JavascriptInterface
        public void save(String name, String mime, String base64) {
            final byte[] data;
            try {
                data = Base64.decode(base64, Base64.DEFAULT);
            } catch (Exception e) {
                return;
            }
            main.post(() -> {
                pendingSave = data;
                Intent i = new Intent(Intent.ACTION_CREATE_DOCUMENT);
                i.addCategory(Intent.CATEGORY_OPENABLE);
                i.setType(mime == null || mime.isEmpty() ? "application/octet-stream" : mime);
                i.putExtra(Intent.EXTRA_TITLE, name);
                try {
                    startActivityForResult(i, SAVE);
                } catch (Exception e) {
                    MainActivity.this.share(name, mime, data);
                }
            });
        }

        @JavascriptInterface
        public void share(String name, String mime, String base64) {
            try {
                final byte[] data = Base64.decode(base64, Base64.DEFAULT);
                main.post(() -> MainActivity.this.share(name, mime, data));
            } catch (Exception ignored) {
            }
        }

        @JavascriptInterface
        public void print(String name, String base64) {
            try {
                final byte[] data = Base64.decode(base64, Base64.DEFAULT);
                main.post(() -> printPdf(name, data));
            } catch (Exception ignored) {
            }
        }

        @JavascriptInterface
        public void printHtml(String name, String html) {
            main.post(() -> printPage(name, html));
        }

        @JavascriptInterface
        public boolean isAndroid() {
            return true;
        }
    }

    private void share(String name, String mime, byte[] data) {
        try {
            File dir = new File(getCacheDir(), "shared");
            //noinspection ResultOfMethodCallIgnored
            dir.mkdirs();
            File f = new File(dir, name.replaceAll("[\\\\/:*?\"<>|]", "_"));
            try (OutputStream out = new FileOutputStream(f)) {
                out.write(data);
            }
            Uri u = FileProvider.getUriForFile(this, getPackageName() + ".files", f);
            Intent i = new Intent(Intent.ACTION_SEND);
            i.setType(mime == null || mime.isEmpty() ? "application/octet-stream" : mime);
            i.putExtra(Intent.EXTRA_STREAM, u);
            i.setClipData(ClipData.newRawUri(name, u));
            i.addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION);
            startActivity(Intent.createChooser(i, name));
        } catch (Exception e) {
            toast("Não foi possível compartilhar o arquivo.");
        }
    }

    /** A print page (HTML with inlined pictures) printed through a WebView of its own. */
    private WebView printView;

    private void printPage(String name, String html) {
        final WebView pv = new WebView(this);
        printView = pv;
        pv.setWebViewClient(new WebViewClient() {
            @Override
            public void onPageFinished(WebView view, String url) {
                PrintManager pm = (PrintManager) getSystemService(Context.PRINT_SERVICE);
                if (pm != null) pm.print(name, view.createPrintDocumentAdapter(name), new PrintAttributes.Builder().build());
            }
        });
        pv.loadDataWithBaseURL(ORIGIN + "/assets/web/", html, "text/html", "UTF-8", null);
    }

    private void printPdf(String name, byte[] pdf) {
        PrintManager pm = (PrintManager) getSystemService(Context.PRINT_SERVICE);
        if (pm == null) return;
        pm.print(name, new PrintDocumentAdapter() {
            @Override
            public void onLayout(PrintAttributes old, PrintAttributes attrs, CancellationSignal cancel, LayoutResultCallback cb, Bundle extras) {
                if (cancel.isCanceled()) {
                    cb.onLayoutCancelled();
                    return;
                }
                cb.onLayoutFinished(new PrintDocumentInfo.Builder(name)
                        .setContentType(PrintDocumentInfo.CONTENT_TYPE_DOCUMENT).build(), true);
            }

            @Override
            public void onWrite(PageRange[] pages, ParcelFileDescriptor dest, CancellationSignal cancel, WriteResultCallback cb) {
                try (OutputStream out = new FileOutputStream(dest.getFileDescriptor())) {
                    out.write(pdf);
                    cb.onWriteFinished(new PageRange[]{PageRange.ALL_PAGES});
                } catch (Exception e) {
                    cb.onWriteFailed(e.getMessage());
                }
            }
        }, null);
    }

    @Override
    protected void onActivityResult(int request, int result, Intent data) {
        super.onActivityResult(request, result, data);
        if (request == PICK) {
            if (pickCallback == null) return;
            Uri[] uris = null;
            if (result == RESULT_OK && data != null) {
                if (data.getClipData() != null) {
                    int n = data.getClipData().getItemCount();
                    uris = new Uri[n];
                    for (int k = 0; k < n; k++) uris[k] = data.getClipData().getItemAt(k).getUri();
                } else if (data.getData() != null) {
                    uris = new Uri[]{data.getData()};
                }
            }
            pickCallback.onReceiveValue(uris);
            pickCallback = null;
        } else if (request == SAVE) {
            byte[] bytes = pendingSave;
            pendingSave = null;
            if (result != RESULT_OK || data == null || data.getData() == null || bytes == null) return;
            try (OutputStream out = getContentResolver().openOutputStream(data.getData(), "wt")) {
                if (out == null) throw new java.io.IOException("no stream");
                out.write(bytes);
                toast("Arquivo salvo.");
            } catch (Exception e) {
                toast("Não foi possível salvar o arquivo.");
            }
        }
    }

    @Override
    public void onBackPressed() {
        // Escape closes dialogs and menus in the app; the system Back leaves only from the top.
        web.evaluateJavascript("window.axionBack ? window.axionBack() : false", value -> {
            if (!"true".equals(value)) MainActivity.super.onBackPressed();
        });
    }

    private void toast(String msg) {
        Toast.makeText(this, msg, Toast.LENGTH_SHORT).show();
    }

    @Override
    protected void onDestroy() {
        if (web != null) web.destroy();
        super.onDestroy();
    }
}
