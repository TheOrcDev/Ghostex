#import <AppKit/AppKit.h>
#import <UniformTypeIdentifiers/UniformTypeIdentifiers.h>
#include <stdbool.h>
#include <stdlib.h>
#include <string.h>

/*
 CDXC:Docs 2026-10-01 WHY:
 The Files view's Open With menu lists the apps Finder would offer, the system default first:
 browsers for the file's link, and the apps registered for its extension (asked by extension
 because a remote file has no local copy). Launch Services is the only source of that list; Rust
 receives one UTF-8 string of `name\tpath` lines and frees it with `ghostex_open_with_free`.
 SEE-ALSO: apps/desktop/src/app/native_docs/open_externally.rs, and
 apps/desktop/src/app/browser_site_requests.rs, which asks it for the app a web page's app link opens.
*/
char *ghostex_open_with_applications(const char *target, bool is_url) {
  @autoreleasepool {
    if (target == NULL) {
      return NULL;
    }
    NSString *text = [NSString stringWithUTF8String:target];
    if (text == nil) {
      return NULL;
    }
    NSWorkspace *workspace = [NSWorkspace sharedWorkspace];
    NSURL *preferred = nil;
    NSArray<NSURL *> *candidates = @[];
    if (is_url) {
      NSURL *url = [NSURL URLWithString:text];
      if (url == nil) {
        return NULL;
      }
      preferred = [workspace URLForApplicationToOpenURL:url];
      candidates = [workspace URLsForApplicationsToOpenURL:url];
    } else {
      UTType *type = [UTType typeWithFilenameExtension:text];
      if (type == nil) {
        return NULL;
      }
      preferred = [workspace URLForApplicationToOpenContentType:type];
      candidates = [workspace URLsForApplicationsToOpenContentType:type];
    }
    NSMutableArray<NSURL *> *apps = [NSMutableArray array];
    if (preferred != nil) {
      [apps addObject:preferred];
    }
    for (NSURL *app in candidates) {
      if (![apps containsObject:app]) {
        [apps addObject:app];
      }
    }
    NSMutableString *lines = [NSMutableString string];
    NSMutableSet<NSString *> *names = [NSMutableSet set];
    for (NSURL *app in apps) {
      NSString *name = [[NSFileManager defaultManager] displayNameAtPath:app.path];
      if ([name.pathExtension isEqualToString:@"app"]) {
        name = name.stringByDeletingPathExtension;
      }
      // The same app installed twice (a beta beside the release) shows once, the preferred copy.
      if (name.length == 0 || [names containsObject:name]) {
        continue;
      }
      [names addObject:name];
      [lines appendFormat:@"%@\t%@\n", name, app.path];
    }
    return strdup(lines.UTF8String);
  }
}

void ghostex_open_with_free(char *lines) { free(lines); }
