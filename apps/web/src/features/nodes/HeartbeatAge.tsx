export function HeartbeatAge({ seconds }: { seconds: number | null }) {
  if (seconds === null) {
    return <span>never</span>;
  }
  if (seconds < 60) {
    return <span>{seconds}s ago</span>;
  }
  const minutes = Math.floor(seconds / 60);
  return <span>{minutes}m ago</span>;
}
