/**
 * What the browser preview remembers across its stand-in clients, which
 * have no shell behind them to share a session with: whether the service
 * mode is on. The session client answers it, the service client sets it,
 * exactly as the shell's session report does for the desktop application.
 */
export const browserDemoState = {
  serviceMode: false,
};
