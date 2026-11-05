const repository = "https://github.com/alexanderwanyoike/openbox";
const assetName = "openbox-x86_64-unknown-linux-gnu.tar.gz";

async function latestRelease() {
  try {
    const response = await fetch(
      "https://api.github.com/repos/alexanderwanyoike/openbox/releases",
      { signal: AbortSignal.timeout(8000) },
    );
    if (!response.ok) return { ok: false, error: "Release check unavailable" };
    const releases = await response.json();
    if (!Array.isArray(releases))
      return { ok: false, error: "Invalid release response" };
    const release = releases.find(
      (value) =>
        !value.draft &&
        !value.prerelease &&
        /^v\d+\.\d+\.\d+$/.test(value.tag_name) &&
        Array.isArray(value.assets) &&
        value.assets.some((asset) => asset.name === assetName),
    );
    return { ok: true, value: release ?? null };
  } catch {
    return { ok: false, error: "Release check unavailable" };
  }
}
const result = await latestRelease();
if (result.ok && result.value) {
  const version = result.value.tag_name;
  const link = document.getElementById("release-link");
  link.textContent = `Download ${version}`;
  link.href = `${repository}/releases/download/${version}/${assetName}`;
  document.getElementById("release-status").textContent =
    `${version} · Linux x86_64`;
}
