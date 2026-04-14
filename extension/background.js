// Minimal MV3 service worker.
// Chrome requires a service_worker entry in manifest.json.
// Future use: extension options page, badge updates.
chrome.runtime.onInstalled.addListener(() => {
  console.log('[vtmtools] Extension installed.');
});
