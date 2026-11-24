// The worker opens the UI; only the visible document owns an animation clock.
chrome.action.onClicked.addListener(() => chrome.runtime.openOptionsPage());
