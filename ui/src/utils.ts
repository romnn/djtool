export const toHHMMSS = (seconds: number) => {
  const hrs = Math.floor(seconds / 3600);
  const mins = Math.floor((seconds - hrs * 3600) / 60);
  const secs = seconds - hrs * 3600 - mins * 60;

  let output = [];
  let parts = [hrs, mins, secs].map((p) => Math.round(p));
  if (parts[0]  == 0) {
    parts = parts.slice(1);
  }
  for (let i = 0; i < parts.length; i++) {
    if (i === 0) {
      output.push(parts[i].toString());
    } else {
      output.push(("00" + parts[i].toString()).slice(-2));
    }
  }
  return output.join(":");
};
