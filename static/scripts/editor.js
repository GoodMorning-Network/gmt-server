function getCookie(name) {
  const value = `; ${document.cookie}`;
  const parts = value.split(`; ${name}=`);
  if (parts.length === 2) return parts.pop().split(";").shift();
}

let compilePath = window.location.pathname.split("/").slice(2).join("/");
let savePath = `tex/${compilePath}`;
console.log(savePath);

document.getElementById("undo").onclick = () => editor.undo();
document.getElementById("redo").onclick = () => editor.redo();

let htmlPreview = document.getElementById("html-preview");
let preview = document.getElementById("preview");
let token = getCookie("token");
let noPreview = document.getElementById("no-preview");

function previewsHideExcept(except) {
  for (let i = 0; i < preview.children.length; i++) {
    if (preview.children[i] == except) {
      preview.children[i].classList.remove("hide");
    } else {
      preview.children[i].classList.add("hide");
    }
  }
}

function preview_url(path) {
  noPreview.classList.add("hide");
  let url = `/api/storage/v1/file/${token}/tex/${path}`;
  let ext = path.split(".").pop();
  switch (ext) {
    case "html":
      fetch(url)
        .then(function (response) {
          return response.text();
        })
        .then(function (response) {
          htmlPreview.innerHTML = response;
          previewsHideExcept(htmlPreview);
        })
        .catch(function (err) {
          console.log("Fetch Error :-S", err);
        });
      break;
    default:
      alert(`Cannot compile .${ext} files into other formats`);
  }
}

switch (previews.length) {
  case 0:
    previewsHideExcept(noPreview);
    break;
  case 1:
    preview_url(previews[0]);
    break;
}

editor.setTheme("ace/theme/monokai");
editor.setOption("wrap", true);
editor.setShowPrintMargin(false);

let file = document.getElementById("file-menu");
let edit = document.getElementById("edit-menu");
let view = document.getElementById("view-menu");
let compile = document.getElementById("compile-menu");

let saveBtn = document.getElementById("save");

function hideAllDropdowns(except) {
  let dropdowns = document.getElementsByClassName("dropdown-content");

  for (let i = 0; i < dropdowns.length; i++) {
    if (dropdowns[i] != except) {
      dropdowns[i].classList.add("hide");
    }
  }
}

document.addEventListener("click", function (event) {
  if (!event.target.parentNode.classList) {
    return;
  }
  if (event.target.parentNode.classList.contains("menubar-item")) {
    hideAllDropdowns(
      event.target.parentNode.getElementsByClassName("dropdown-content")[0]
    );
  }
  hideAllDropdowns(
    event.target.parentNode.getElementsByClassName("dropdown-content")[0]
  );
});

function showFile() {
  file.getElementsByClassName("dropdown-content")[0].classList.remove("hide");
}
function showEdit() {
  edit.getElementsByClassName("dropdown-content")[0].classList.remove("hide");
}
function showView() {
  view.getElementsByClassName("dropdown-content")[0].classList.remove("hide");
}
function showCompile() {
  compile
    .getElementsByClassName("dropdown-content")[0]
    .classList.remove("hide");
}
document.getElementById("file").onclick = showFile;
document.getElementById("edit").onclick = showEdit;
document.getElementById("view").onclick = showView;
document.getElementById("compile").onclick = showCompile;

let modes = view.getElementsByClassName("dropdown-content")[0].childNodes;

for (let i = 0; i < modes.length; i++) {
  modes[i].onclick = () => {
    editor.setKeyboardHandler(`ace/keyboard/${modes[i].getAttribute("mode")}`);
    localStorage.setItem("editor-mode", modes[i].getAttribute("mode"));
  };
}

if (localStorage.getItem("editor-mode")) {
  editor.setKeyboardHandler(
    `ace/keyboard/${localStorage.getItem("editor-mode")}`
  );
}

function addRunning(element) {
  element.classList.add("running");
  element.parentNode.parentNode.classList.add("running");
}

function removeRunning(element) {
  element.classList.remove("running");
  if (element.parentNode.getElementsByClassName("running").length !== 0) {
    return;
  }

  element.parentNode.parentNode.classList.remove("running");
}

function save(f) {
  if (saveBtn.classList.contains("running")) {
    return;
  }
  addRunning(saveBtn);
  const data = new FormData();
  const blob = new Blob([editor.getValue()], {
    type: "text/plain",
  });
  data.append("file", blob);

  const xhr = new XMLHttpRequest();

  xhr.open("POST", `/api/storage/v1/upload-overwrite/${token}/${savePath}`);
  xhr.send(data);
  xhr.onreadystatechange = function () {
    if (xhr.readyState === 4) {
      try {
        let res = JSON.parse(xhr.responseText);
        if (res.type === "error") {
          alert(`There was an error saving this file: ${JSON.stringify(res.kind)}`);
        }
      } catch (_) {}
      removeRunning(saveBtn);
      if (typeof f == "function") {
        f();
      }
    }
  };
}

saveBtn.onclick = save;

function compileFile(target, btn) {
  let url = "/api/tex/compile/v1/simple";
  let data = {
    token,
    path: compilePath,
    from: thisFormat,
    to: target,
  };
  fetch(url, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
    },
    body: JSON.stringify(data),
  })
    .then((response) => response.json())
    .then((data) => {
      removeRunning(btn);
      if (data.type == "error") {
        alert(`Error compiling: ${JSON.stringify(data.kind)}`);
        return;
      }
      preview_url(data.newpath);
    })
    .catch((error) => console.error(error));
}

let compiles = document.querySelectorAll("#compile-menu .dropdown-item");
for (let i = 0; i < compiles.length; i++) {
  compiles[i].onclick = () =>
    save(() => {
      if (compiles[i].classList.contains("running")) {
        return;
      }
      addRunning(compiles[i]);
      compileFile(compiles[i].getAttribute("target", compiles[i]), compiles[i]);
    });
}
