// Copied from apps/mobile/app/plugins/withSceneLifecycle.js (same reason: the iOS 27 SDK
// requires the UIScene lifecycle, and neither RN 0.86 nor Expo SDK 57 adopt it).
// Apps built with the iOS 27 SDK trap at launch unless they adopt the UIScene
// lifecycle (UIApplicationEvaluateRuntimeIssueForNoSceneLifecycleAdoption).
// Neither RN 0.86 nor Expo SDK 57 ship scene support, so inject it:
//  - SceneDelegate.swift that creates the window and starts React Native
//  - AppDelegate patched to cache launchOptions and not create the window
//  - UIApplicationSceneManifest in Info.plist
const { withDangerousMod, withInfoPlist, withXcodeProject, IOSConfig } = require('expo/config-plugins');
const fs = require('fs');
const path = require('path');

const SCENE_DELEGATE = `import UIKit
import React

class SceneDelegate: UIResponder, UIWindowSceneDelegate {
  var window: UIWindow?

  func scene(
    _ scene: UIScene,
    willConnectTo session: UISceneSession,
    options connectionOptions: UIScene.ConnectionOptions
  ) {
    guard let windowScene = scene as? UIWindowScene else { return }
    guard let appDelegate = UIApplication.shared.delegate as? AppDelegate,
          let factory = appDelegate.reactNativeFactory else { return }
    let window = UIWindow(windowScene: windowScene)
    factory.startReactNative(
      withModuleName: "main",
      in: window,
      launchOptions: appDelegate.cachedLaunchOptions)
    appDelegate.window = window
    self.window = window

    open(connectionOptions.urlContexts)
  }

  func scene(_ scene: UIScene, openURLContexts URLContexts: Set<UIOpenURLContext>) {
    open(URLContexts)
  }

  private func open(_ URLContexts: Set<UIOpenURLContext>) {
    guard let appDelegate = UIApplication.shared.delegate as? AppDelegate else { return }
    for context in URLContexts {
      _ = appDelegate.application(UIApplication.shared, open: context.url, options: [:])
    }
  }
}
`;

function patchAppDelegate(contents) {
  if (contents.includes('cachedLaunchOptions')) return contents; // already patched
  let out = contents.replace(
    /#if os\(iOS\) \|\| os\(tvOS\)\n\s*window = UIWindow\(frame: UIScreen\.main\.bounds\)\n\s*factory\.startReactNative\(\n\s*withModuleName: "main",\n\s*in: window,\n\s*launchOptions: launchOptions\)\n#endif\n/,
    '    cachedLaunchOptions = launchOptions\n'
  );
  out = out.replace(
    /var window: UIWindow\?\n/,
    'var window: UIWindow?\n  var cachedLaunchOptions: [UIApplication.LaunchOptionsKey: Any]?\n'
  );
  if (!out.includes('cachedLaunchOptions')) {
    throw new Error('withSceneLifecycle: AppDelegate.swift did not match the expected template');
  }
  return out;
}

module.exports = function withSceneLifecycle(config) {
  config = withDangerousMod(config, [
    'ios',
    (cfg) => {
      const projectName = cfg.modRequest.projectName;
      const appDir = path.join(cfg.modRequest.platformProjectRoot, projectName);
      fs.writeFileSync(path.join(appDir, 'SceneDelegate.swift'), SCENE_DELEGATE);
      const appDelegatePath = path.join(appDir, 'AppDelegate.swift');
      fs.writeFileSync(appDelegatePath, patchAppDelegate(fs.readFileSync(appDelegatePath, 'utf8')));
      return cfg;
    },
  ]);

  config = withXcodeProject(config, (cfg) => {
    const projectName = cfg.modRequest.projectName;
    IOSConfig.XcodeUtils.addBuildSourceFileToGroup({
      filepath: `${projectName}/SceneDelegate.swift`,
      groupName: projectName,
      project: cfg.modResults,
    });
    return cfg;
  });

  config = withInfoPlist(config, (cfg) => {
    cfg.modResults.UIApplicationSceneManifest = {
      UIApplicationSupportsMultipleScenes: false,
      UISceneConfigurations: {
        UIWindowSceneSessionRoleApplication: [
          {
            UISceneConfigurationName: 'Default',
            UISceneDelegateClassName: '$(PRODUCT_MODULE_NAME).SceneDelegate',
          },
        ],
      },
    };
    return cfg;
  });

  return config;
};
