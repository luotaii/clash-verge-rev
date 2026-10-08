import path from 'node:path'

const SERVICE_URL_PREFIX =
  'https://github.com/clash-verge-rev/clash-verge-service-ipc/releases/download'

export function resolvePinnedServiceManifest(
  cargoManifest,
  metadata,
  applicationManifestPath,
) {
  const dependency = cargoManifest
    .split(/\r?\n/)
    .find((line) => line.trimStart().startsWith('clash_verge_service_ipc ='))
  const revision = dependency?.match(/\brev\s*=\s*"([^"]+)"/)?.[1]
  const repository = dependency?.match(/\bgit\s*=\s*"([^"]+)"/)?.[1]
  if (!repository || !revision || !/^[0-9a-f]{40}$/.test(revision)) {
    throw new Error('The bundled service must use a full Git commit revision')
  }

  const application = metadata.packages.find(
    (pkg) =>
      path.resolve(pkg.manifest_path) === path.resolve(applicationManifestPath),
  )
  const serviceId = metadata.resolve?.nodes
    .find((node) => node.id === application?.id)
    ?.deps.find((dep) => dep.name === 'clash_verge_service_ipc')?.pkg
  const service = metadata.packages.find((pkg) => pkg.id === serviceId)
  if (
    service?.source !== `git+${repository}?rev=${revision}#${revision}` ||
    !Object.hasOwn(service.features, 'forwarding-guard')
  ) {
    throw new Error('The resolved service does not match the pinned dependency')
  }
  return service.manifest_path
}

export function resolveServiceRelease(cargoManifest, host, platform) {
  const dependency = cargoManifest
    .split(/\r?\n/)
    .find((line) => line.trimStart().startsWith('clash_verge_service_ipc ='))
  const packageVersion = dependency?.match(/\bversion\s*=\s*"([^"]+)"/)?.[1]
  if (!packageVersion) {
    throw new Error(
      'clash_verge_service_ipc dependency must declare an inline version',
    )
  }

  const version = `v${packageVersion}`
  const archiveExt = platform === 'win32' ? 'zip' : 'tar.gz'
  const archiveFile = `clash-verge-service-ipc-${version}-${host}.${archiveExt}`
  return {
    version,
    archiveFile,
    downloadURL: `${SERVICE_URL_PREFIX}/${version}/${archiveFile}`,
  }
}
