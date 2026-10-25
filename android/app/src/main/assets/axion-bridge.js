// Axion Android bridge: injected into the web build once the page has loaded.
// - Downloads (an <a download> on a blob: URL) open the system "Save as" dialog.
// - Printing (a PDF or a print page in a hidden iframe) goes to Android printing.
// - Files opened in or shared with Axion arrive as a dropped file (window.axionOpenFile).
// - The Back button closes the open dialog or menu (Escape); twice quickly leaves the app.
(function () {
  if (window.__axionBridge) return;
  window.__axionBridge = true;
  var A = window.AxionAndroid;
  if (!A) return;

  // blob: URL -> Blob, kept for a minute so a URL revoked right after click() can still be read.
  var blobs = new Map();
  var create = URL.createObjectURL.bind(URL);
  URL.createObjectURL = function (obj) {
    var url = create(obj);
    if (obj instanceof Blob) {
      blobs.set(url, obj);
      setTimeout(function () { blobs.delete(url); }, 60000);
    }
    return url;
  };

  function toBase64(blob) {
    return new Promise(function (resolve, reject) {
      var r = new FileReader();
      r.onload = function () {
        var s = String(r.result || "");
        resolve(s.slice(s.indexOf(",") + 1));
      };
      r.onerror = function () { reject(r.error); };
      r.readAsDataURL(blob);
    });
  }

  var mimeByExt = {
    pdf: "application/pdf",
    docx: "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
    doc: "application/msword",
    odt: "application/vnd.oasis.opendocument.text",
    rtf: "application/rtf",
    md: "text/markdown",
    txt: "text/plain",
    html: "text/html",
    htm: "text/html",
    tex: "application/x-tex",
    pptx: "application/vnd.openxmlformats-officedocument.presentationml.presentation",
    png: "image/png",
    jpg: "image/jpeg",
    jpeg: "image/jpeg",
    tif: "image/tiff",
    tiff: "image/tiff",
    csv: "text/csv",
    xfdf: "application/vnd.adobe.xfdf",
    fdf: "application/vnd.fdf",
    zip: "application/zip"
  };
  function mimeFor(name, blob) {
    var ext = (name.split(".").pop() || "").toLowerCase();
    return mimeByExt[ext] || (blob && blob.type) || "application/octet-stream";
  }

  function saveBlob(name, blob) {
    toBase64(blob).then(function (b64) { A.save(name, mimeFor(name, blob), b64); });
  }

  var click = HTMLAnchorElement.prototype.click;
  HTMLAnchorElement.prototype.click = function () {
    var href = this.href || "";
    var blob = blobs.get(href);
    if (blob && this.hasAttribute("download")) {
      saveBlob(this.getAttribute("download") || "documento", blob);
      return;
    }
    if (href.indexOf("data:") === 0 && this.hasAttribute("download")) {
      var name = this.getAttribute("download") || "documento";
      var comma = href.indexOf(",");
      var meta = href.slice(5, comma);
      var data = href.slice(comma + 1);
      var b64 = meta.indexOf(";base64") >= 0 ? data : btoa(unescape(encodeURIComponent(decodeURIComponent(data))));
      A.save(name, mimeFor(name, null), b64);
      return;
    }
    return click.apply(this, arguments);
  };

  // Printing from a hidden iframe: a PDF blob prints as it is; an HTML print page is sent with
  // its pictures inlined and printed by Android's own WebView printing.
  function inlineImages(doc) {
    var imgs = Array.prototype.slice.call(doc.images || []);
    return Promise.all(imgs.map(function (img) {
      var b = blobs.get(img.src);
      if (!b) return Promise.resolve();
      return toBase64(b).then(function (b64) { img.setAttribute("src", "data:" + (b.type || "image/png") + ";base64," + b64); });
    }));
  }
  function printFrame(frame) {
    var src = frame.getAttribute("src") || "";
    var blob = blobs.get(src);
    var title = (document.title || "Axion") + ".pdf";
    if (blob && (blob.type === "application/pdf" || !blob.type)) {
      toBase64(blob).then(function (b64) { A.print(title, b64); });
      return;
    }
    var doc = frame.contentDocument;
    if (!doc) return;
    inlineImages(doc).then(function () {
      A.printHtml(document.title || "Axion", "<!doctype html>" + doc.documentElement.outerHTML);
    });
  }
  function hookFrame(frame) {
    var hook = function () {
      try {
        if (frame.contentWindow && !frame.contentWindow.__axionPrint) {
          frame.contentWindow.__axionPrint = true;
          frame.contentWindow.print = function () { printFrame(frame); };
        }
      } catch (e) { /* cross-origin: leave it */ }
    };
    hook();
    frame.addEventListener("load", hook, true);
  }
  new MutationObserver(function (records) {
    records.forEach(function (r) {
      r.addedNodes.forEach(function (n) {
        if (n.tagName === "IFRAME") hookFrame(n);
        else if (n.querySelectorAll) n.querySelectorAll("iframe").forEach(hookFrame);
      });
    });
  }).observe(document.documentElement, { childList: true, subtree: true });
  document.querySelectorAll("iframe").forEach(hookFrame);
  window.print = function () {
    var f = document.querySelector("iframe");
    if (f) printFrame(f);
  };

  // A file opened in or shared with Axion: dropped on the app, as a file dragged in would be.
  window.axionOpenFile = function (name, b64) {
    var bin = atob(b64);
    var bytes = new Uint8Array(bin.length);
    for (var i = 0; i < bin.length; i++) bytes[i] = bin.charCodeAt(i);
    var file = new File([bytes], name, { type: mimeFor(name, null) });
    var target = document.querySelector("canvas") || document.body;
    var dt = new DataTransfer();
    dt.items.add(file);
    ["dragenter", "dragover", "drop"].forEach(function (type) {
      var ev = new DragEvent(type, { bubbles: true, cancelable: true, dataTransfer: dt });
      target.dispatchEvent(ev);
    });
  };

  // Back: Escape for the app (closes dialogs, menus, panels); a second Back within 2 s leaves.
  var lastBack = 0;
  window.axionBack = function () {
    var now = Date.now();
    if (now - lastBack < 2000) return false;
    lastBack = now;
    var target = document.querySelector("canvas") || document.body;
    ["keydown", "keyup"].forEach(function (type) {
      target.dispatchEvent(new KeyboardEvent(type, { key: "Escape", code: "Escape", keyCode: 27, bubbles: true }));
    });
    return true;
  };
})();
