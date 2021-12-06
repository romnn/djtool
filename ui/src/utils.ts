export const toHHMMSS = (seconds: number) => {
  const hrs = Math.floor(seconds / 3600);
  const mins = Math.floor((seconds - hrs * 3600) / 60);
  const secs = seconds - hrs * 3600 - mins * 60;

  const hrsString = ("00" + Math.round(hrs).toString()).slice(-2);
  const minsString = ("00" + Math.round(mins).toString()).slice(-2);
  const secsString = ("00" + Math.round(secs).toString()).slice(-2);
  return `${hrsString}:${minsString}:${secsString}`;
};
