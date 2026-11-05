const repository = "https://github.com/alexanderwanyoike/openbox";
const links = [...document.querySelectorAll("[data-asset]")];
const assetNames = new Set(links.map((link) => link.dataset.asset));

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
        value.assets.some((asset) => assetNames.has(asset.name)),
    );
    return { ok: true, value: release ?? null };
  } catch {
    return { ok: false, error: "Release check unavailable" };
  }
}
const result = await latestRelease();
if (result.ok && result.value) {
  const version = result.value.tag_name;
  for (const link of links) {
    const { asset, platform, label } = link.dataset;
    if (result.value.assets.some((value) => value.name === asset)) {
      link.textContent = label;
      link.setAttribute("aria-label", `Download ${platform} ${version}`);
      link.href = `${repository}/releases/download/${version}/${asset}`;
    }
  }
  document.getElementById("release-status").textContent = version;
}
