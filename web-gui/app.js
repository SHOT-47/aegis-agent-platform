const ACTIVITY_STATES = [
  "Initializing", "UnderstandingRequest", "Planning", "SearchingProject",
  "Indexing", "RetrievingContext", "PreparingModelRequest", "RunningInference",
  "ExecutingTool", "RunningCommand", "WaitingForApproval", "BrowsingWebsite",
  "ScrapingPages", "DelegatingAgent", "RunningTests", "VerifyingChanges",
  "RecoveringFromError", "Paused", "Blocked", "Completed", "Failed", "Canceled"
];
document.querySelectorAll(".tab-btn").forEach(btn => {
  btn.addEventListener("click", () => {
    document.querySelectorAll(".tab-btn").forEach(b => b.classList.remove("active"));
    document.querySelectorAll(".workspace-panel").forEach(p => p.classList.remove("active"));
    btn.classList.add("active");
    document.getElementById(btn.dataset.workspace).classList.add("active");
  });
});
document.getElementById("state-grid").innerHTML = ACTIVITY_STATES.map(s => `<span class="state-chip">${s}</span>`).join("");
