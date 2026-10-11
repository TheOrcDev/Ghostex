#import <AppKit/AppKit.h>
#include <math.h>
#include <stdbool.h>
#include <stdint.h>

/*
 CDXC:Settings 2026-09-28 WHY:
 The native Settings colour swatch opens the shared NSColorPanel, the panel Chromium showed for the
 React `<input type="color">` (ColorPanelCocoa): no alpha, every colour change reported while it is
 open, and the chooser ends when the panel closes or another field takes it over. Rust owns the
 context; `finished` is sent exactly once so it can release it.
 SEE-ALSO: apps/desktop/src/app/settings_modal_lifecycle.rs.
*/
typedef void (*GhostexColorPanelCallback)(void *context, uint32_t rgb, bool finished);

@interface GhostexColorPanelTarget : NSObject
@property(nonatomic) void *context;
@property(nonatomic) GhostexColorPanelCallback callback;
- (void)finish;
@end

static GhostexColorPanelTarget *gGhostexColorPanelTarget = nil;

static uint32_t GhostexColorPanelRGB(NSColor *color) {
  NSColor *srgb = [color colorUsingColorSpace:[NSColorSpace sRGBColorSpace]];
  if (srgb == nil) {
    return 0;
  }
  CGFloat red = 0, green = 0, blue = 0, alpha = 0;
  [srgb getRed:&red green:&green blue:&blue alpha:&alpha];
  uint32_t r = (uint32_t)lround(fmin(fmax(red, 0.0), 1.0) * 255.0);
  uint32_t g = (uint32_t)lround(fmin(fmax(green, 0.0), 1.0) * 255.0);
  uint32_t b = (uint32_t)lround(fmin(fmax(blue, 0.0), 1.0) * 255.0);
  return (r << 16) | (g << 8) | b;
}

@implementation GhostexColorPanelTarget
- (void)colorChanged:(NSColorPanel *)panel {
  if (self.callback != NULL) {
    self.callback(self.context, GhostexColorPanelRGB(panel.color), false);
  }
}

- (void)panelWillClose:(NSNotification *)notification {
  [self finish];
}

- (void)finish {
  [[NSNotificationCenter defaultCenter] removeObserver:self];
  NSColorPanel *panel = [NSColorPanel sharedColorPanel];
  // NSColorPanel has `setTarget:` but no readable `target`; the panel is ours while this object is
  // the current chooser.
  if (gGhostexColorPanelTarget == self) {
    [panel setTarget:nil];
    [panel setAction:NULL];
  }
  GhostexColorPanelCallback callback = self.callback;
  void *context = self.context;
  self.callback = NULL;
  self.context = NULL;
  if (gGhostexColorPanelTarget == self) {
    gGhostexColorPanelTarget = nil;
  }
  if (callback != NULL) {
    callback(context, 0, true);
  }
}
@end

void GhostexGpuiShowColorPanel(uint32_t rgb, void *context, GhostexColorPanelCallback callback) {
  if (gGhostexColorPanelTarget != nil) {
    [gGhostexColorPanelTarget finish];
  }
  GhostexColorPanelTarget *target = [GhostexColorPanelTarget new];
  target.context = context;
  target.callback = callback;
  gGhostexColorPanelTarget = target;

  NSColorPanel *panel = [NSColorPanel sharedColorPanel];
  [panel setShowsAlpha:NO];
  // Seeding the colour must not report it back as a pick.
  [panel setTarget:nil];
  [panel setAction:NULL];
  [panel setColor:[NSColor colorWithSRGBRed:((rgb >> 16) & 0xff) / 255.0
                                      green:((rgb >> 8) & 0xff) / 255.0
                                       blue:(rgb & 0xff) / 255.0
                                      alpha:1.0]];
  [panel setTarget:target];
  [panel setAction:@selector(colorChanged:)];
  [[NSNotificationCenter defaultCenter] addObserver:target
                                           selector:@selector(panelWillClose:)
                                               name:NSWindowWillCloseNotification
                                             object:panel];
  [panel makeKeyAndOrderFront:nil];
}
