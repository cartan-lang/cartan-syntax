"use strict";

const form = document.querySelector("[data-inspect-form]");
if (form) {
  const button = form.querySelector("button");
  const pending = document.querySelector("[data-inspect-pending]");
  form.addEventListener("submit", () => {
    button.disabled = true;
    button.textContent = "Inspecting…";
    form.setAttribute("aria-busy", "true");
    pending.hidden = false;
  });
  // A browser may restore the submitted page from its back/forward cache.
  window.addEventListener("pageshow", () => {
    button.disabled = false;
    button.textContent = "Inspect source";
    form.removeAttribute("aria-busy");
    pending.hidden = true;
  });
}
