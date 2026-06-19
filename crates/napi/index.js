'use strict'

const { existsSync } = require('fs')
const { join } = require('path')

const { platform, arch } = process

let nativeBinding = null
let loadError = null

function isMusl() {
  const report = process.report && process.report.getReport()
  if (report && typeof report === 'object') {
    if (report.header && report.header.glibcVersionRuntime) {
      return false
    }
  }
  return true
}

switch (platform) {
  case 'win32':
    if (arch !== 'x64') {
      throw new Error(`Unsupported Windows architecture: ${arch}`)
    }
    try {
      if (existsSync(join(__dirname, 'rinhook.win32-x64-msvc.node'))) {
        nativeBinding = require('./rinhook.win32-x64-msvc.node')
      } else {
        nativeBinding = require('@qdrx/rinhook-win32-x64-msvc')
      }
    } catch (e) {
      loadError = e
    }
    break

  case 'darwin':
    if (arch !== 'arm64') {
      throw new Error(`Unsupported macOS architecture: ${arch}`)
    }
    try {
      if (existsSync(join(__dirname, 'rinhook.darwin-arm64.node'))) {
        nativeBinding = require('./rinhook.darwin-arm64.node')
      } else {
        nativeBinding = require('@qdrx/rinhook-darwin-arm64')
      }
    } catch (e) {
      loadError = e
    }
    break

  case 'linux':
    if (arch !== 'x64') {
      throw new Error(`Unsupported Linux architecture: ${arch}`)
    }
    if (isMusl()) {
      throw new Error('Linux musl is not supported')
    }
    try {
      if (existsSync(join(__dirname, 'rinhook.linux-x64-gnu.node'))) {
        nativeBinding = require('./rinhook.linux-x64-gnu.node')
      } else {
        nativeBinding = require('@qdrx/rinhook-linux-x64-gnu')
      }
    } catch (e) {
      loadError = e
    }
    break

  default:
    throw new Error(`Unsupported platform: ${platform}`)
}

if (!nativeBinding) {
  if (loadError) throw loadError
  throw new Error('Failed to load native binding')
}

module.exports.startListening = nativeBinding.startListening
module.exports.stopListening = nativeBinding.stopListening
