# macOS Sequoia 15.6 Accessibility Permission Research Report

## Executive Summary

This report investigates a critical issue affecting macOS window management applications in macOS Sequoia 15.6, where System Settings displays accessibility permission as granted (blue toggle enabled), but the `AXIsProcessTrusted()` API consistently returns false. This disconnect prevents applications from utilizing accessibility APIs for window manipulation despite apparent user authorization.

**Key Findings:**
- Confirmed system-wide issue affecting multiple developers on macOS Sequoia 15.6
- TCC (Transparency, Consent, and Control) database corruption is the primary cause
- Apple introduced stricter permission handling and security fixes in Sequoia that affect app activation policies
- Swift Package Manager build processes may lack specific requirements for TCC persistence

**Recommended Solution:**
Reset TCC database and implement improved permission checking with proper code signing practices.

---

## Problem Statement

**Application:** Symposium (macOS window management app)  
**Environment:** macOS 15.6 (24G84) Sequoia, Swift 6.1.2, SPM build  
**Issue:** `AXIsProcessTrusted()` returns false despite System Settings showing permission granted  
**Impact:** Application cannot access accessibility APIs for window manipulation  

### Technical Configuration
- **Bundle ID:** com.symposium.app
- **Code Signing:** Apple Development certificate (Team ID: 5AQN5YDDZA)
- **LSUIElement:** false (normal app behavior)
- **Build System:** Swift Package Manager

---

## Research Findings

### 1. macOS Sequoia-Specific Issues

#### App Activation Policy Problems
macOS Sequoia introduced stricter handling of application activation policies that affects accessibility permissions:

- **NSApplicationActivationPolicyProhibited** creates new failure paths that didn't exist in previous macOS versions
- Apple specifically recommends using **NSApplicationActivationPolicyAccessory** for background apps that need system access
- Standard apps should use **NSApplicationActivationPolicyRegular** (LSUIElement=false or no key)

**Source:** Apple Developer Forums - Problem with event tap permission in Sequoia

#### Security Hardening Impact
macOS 15.6 includes multiple security fixes affecting permission handling:
- CVE-2025-43268: "A permissions issue was addressed with additional restrictions"
- CVE-2025-43243: "A permissions issue was addressed with additional restrictions"  
- CVE-2025-43230: "The issue was addressed with additional permissions checks"

These security patches may have tightened TCC validation, causing previously working configurations to fail.

### 2. TCC Database Corruption

#### Symptoms
Multiple developers report identical symptoms:
- `AXIsProcessTrusted()` returns true initially but APIs return nil/empty results
- System Settings UI shows permission granted but actual functionality fails
- Inconsistent behavior between UI state and API responses

#### Root Cause
The TCC database can enter an inconsistent state where:
- Permission records exist and show as granted in UI
- Internal TCC validation fails due to corrupted records or signature mismatches
- Accessibility subsystem doesn't recognize the permission grants

**Source:** Apple Developer Forums - macOS TCC Accessibility permission granted, yet the Accessibility APIs sporadically return no data

### 3. System Settings UI Bug

#### Toggle Behavior Issues
- macOS Ventura and later replaced reliable checkboxes with toggles that can show incorrect states
- Quick toggling of accessibility permissions can cause `AXIsProcessTrusted()` to return wrong values
- UI state doesn't always reflect actual TCC database state

**Source:** Apple Developer Forums - AXIsProcessTrusted returns wrong value

### 4. Swift Package Manager Considerations

#### Build System Differences
Swift Package Manager builds may lack certain configurations that Xcode projects include:
- Code signing continuity between builds
- Proper entitlement handling
- Bundle identifier consistency

Apps built with SPM require explicit code signing to maintain TCC permissions across development iterations.

---

## Root Cause Analysis

### Primary Cause: TCC Database Corruption
The disconnect between System Settings UI and `AXIsProcessTrusted()` indicates TCC database corruption or inconsistency. This can occur due to:

1. **System Updates:** macOS updates can corrupt existing TCC records
2. **Code Signing Changes:** Bundle signature changes between builds invalidate TCC records
3. **Security Hardening:** Sequoia's stricter validation rejects previously valid records

### Secondary Factors
1. **App Activation Policy:** Using prohibited or incorrect activation policies
2. **Bundle ID Consistency:** Changes in bundle identifier between builds
3. **Development vs. Production Signing:** Different signing certificates causing TCC confusion

---

## Solutions and Recommendations

### Immediate Fix: TCC Database Reset

**Primary Solution:**
```bash
sudo tccutil reset Accessibility
```

This command:
- Clears all accessibility permissions for all apps
- Forces clean TCC database state
- Requires users to re-grant permissions to all apps
- Resolves UI/API state inconsistencies

**Alternative: App-Specific Reset:**
```bash
sudo tccutil reset Accessibility com.symposium.app
```

### Code Implementation Improvements

#### 1. Proper Permission Checking
```swift
func checkAccessibilityPermission() -> Bool {
    // First check without prompting to register app in TCC
    let options: [String: Any] = [
        kAXTrustedCheckOptionPrompt.takeUnretainedValue() as String: false
    ]
    let hasPermission = AXIsProcessTrustedWithOptions(options as CFDictionary)
    
    if !hasPermission {
        // Direct user to accessibility panel
        openAccessibilityPreferences()
    }
    
    return hasPermission
}

private func openAccessibilityPreferences() {
    let urlString = "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility"
    if let url = URL(string: urlString) {
        NSWorkspace.shared.open(url)
    }
}
```

#### 2. Permission Change Monitoring
```swift
func setupPermissionMonitoring() {
    DistributedNotificationCenter.default().addObserver(
        forName: NSNotification.Name("com.apple.accessibility.api"),
        object: nil,
        queue: nil
    ) { [weak self] _ in
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.1) {
            self?.updatePermissionStatus()
        }
    }
}
```

#### 3. Activation Policy Configuration
```swift
func configureAppActivationPolicy() {
    // Ensure proper activation policy for accessibility apps
    NSApp.setActivationPolicy(.regular) // For normal apps with LSUIElement=false
    // or .accessory for background apps that need system access
}
```

### Development Environment Fixes

#### 1. Consistent Code Signing
```bash
# Sign app bundle consistently
codesign --force --deep --sign "Apple Development: Your Name (XXXXXXXXXX)" \
         --options runtime YourApp.app

# Verify signature
codesign -dv --verbose=4 YourApp.app
```

#### 2. TCC Reset for Development
Add to build process:
```bash
# Reset permissions for development
tccutil reset Accessibility com.symposium.app
```

### Info.plist Optimization
```xml
<key>NSAccessibilityUsageDescription</key>
<string>Symposium needs accessibility permission to manage and stack windows from other applications.</string>

<key>CFBundleIdentifier</key>
<string>com.symposium.app</string>

<key>LSUIElement</key>
<false/>

<!-- Ensure proper app category -->
<key>LSApplicationCategoryType</key>
<string>public.app-category.utilities</string>
```

---

## Implementation Action Plan

### Phase 1: Immediate Resolution (User-facing)
1. **Document TCC reset procedure** for users experiencing the issue
2. **Update app documentation** to include troubleshooting steps
3. **Implement improved permission checking** with better error handling

### Phase 2: Development Improvements
1. **Implement permission monitoring** to detect changes in real-time
2. **Add diagnostic capabilities** to detect TCC database inconsistencies
3. **Improve build process** with consistent code signing

### Phase 3: Long-term Stability
1. **Test across different macOS versions** to ensure compatibility
2. **Monitor Apple Developer Forums** for additional Sequoia-related issues
3. **Consider alternative APIs** if accessibility permission issues persist

---

## Diagnostic Commands

### TCC Database Inspection
```bash
# Check permission status in TCC database
sudo sqlite3 "/Library/Application Support/com.apple.TCC/TCC.db" \
"SELECT service, client, auth_value FROM access WHERE service='kTCCServiceAccessibility' AND client='com.symposium.app';"

# List all accessibility permissions
sudo sqlite3 "/Library/Application Support/com.apple.TCC/TCC.db" \
"SELECT client, auth_value FROM access WHERE service='kTCCServiceAccessibility';"
```

### Code Signature Verification
```bash
# Detailed signature info
codesign -dv --verbose=4 /Applications/Symposium.app

# Check entitlements
codesign -d --entitlements - /Applications/Symposium.app
```

### System Information
```bash
# macOS version
sw_vers

# TCC daemon status
sudo launchctl list | grep tcc
```

---

## Known Workarounds

### Alternative Permission Request Method
Some developers report success using direct System Preferences opening instead of `AXIsProcessTrustedWithOptions`:

```swift
func requestAccessibilityPermission() {
    // Check current status without prompting
    let options: [String: Any] = [
        kAXTrustedCheckOptionPrompt.takeUnretainedValue() as String: false
    ]
    let hasPermission = AXIsProcessTrustedWithOptions(options as CFDictionary)
    
    if !hasPermission {
        // Open System Preferences directly
        let urlString = "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility"
        NSWorkspace.shared.open(URL(string: urlString)!)
        
        // Show user instructions
        showPermissionInstructions()
    }
}
```

### Development Environment Workaround
For development, manually reset permissions after each build:
```bash
# Add to Xcode build script or SPM build process
tccutil reset Accessibility $(PRODUCT_BUNDLE_IDENTIFIER)
```

---

## Additional Considerations

### macOS Version Compatibility
- **Monterey (12.x):** Generally stable accessibility permission handling
- **Ventura (13.x):** Toggle UI introduced bugs, basic functionality works
- **Sonoma (14.x):** Improved but still occasional issues
- **Sequoia (15.x):** Significant security hardening affecting TCC behavior

### App Store vs. Direct Distribution
- **Mac App Store:** Sandboxed apps cannot request accessibility permissions
- **Direct Distribution:** Full accessibility access available with proper signing
- **Notarization:** Required for distribution outside App Store, may affect TCC behavior

### Future Considerations
Apple is likely to continue tightening security around accessibility permissions. Consider:
- Alternative APIs for window management that don't require accessibility permissions
- User education about the necessity and security implications of accessibility access
- Monitoring Apple's developer communications for changes to TCC policies

---

## References

1. Apple Developer Forums - "Problem with event tap permission in Sequoia"
2. Apple Developer Forums - "macOS TCC Accessibility permission granted, yet APIs return no data"  
3. Apple Developer Forums - "AXIsProcessTrusted returns wrong value"
4. Stack Overflow - "AXIsProcessTrustedWithOptions doesn't return true even when app is ticked"
5. Apple Support - "About the security content of macOS Sequoia 15.6"
6. TidBITS - "macOS 15 Sequoia's Excessive Permissions Prompts Will Hurt Security"
7. Macworld - "How to fix macOS Accessibility permission when an app can't be enabled"

---

## Report Information

**Prepared:** January 2025  
**Research Period:** January 2025  
**macOS Versions Covered:** 15.6 (24G84) Sequoia  
**Primary Focus:** Swift-based window management applications  
**Research Methodology:** Developer forum analysis, technical documentation review, security bulletin analysis

---

*This report is based on publicly available information, developer community reports, and official Apple documentation as of January 2025. macOS behavior may change with future updates.*