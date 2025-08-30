import SwiftUI
import AppKit

struct ContentView: View {
    @StateObject private var cgsManager = CGSWindowManager()
    @State private var searchText: String = ""
    @State private var customLevel: String = "0"
    @State private var alphaValue: Float = 1.0
    
    var filteredWindows: [CGSWindowManager.TestWindowInfo] {
        if searchText.isEmpty {
            return cgsManager.allWindows
        } else {
            return cgsManager.allWindows.filter { window in
                window.displayName.localizedCaseInsensitiveContains(searchText) ||
                window.appName.localizedCaseInsensitiveContains(searchText) ||
                window.title.localizedCaseInsensitiveContains(searchText)
            }
        }
    }
    
    var body: some View {
        HStack(spacing: 0) {
            // Left panel: Window list and controls
            VStack(spacing: 16) {
                // Status section
                VStack(spacing: 8) {
                    HStack {
                        Image(systemName: cgsManager.hasAccessibilityPermission ? "checkmark.circle.fill" : "exclamationmark.triangle.fill")
                            .foregroundColor(cgsManager.hasAccessibilityPermission ? .green : .orange)
                        Text("CGS Window Testing")
                            .font(.headline)
                        Spacer()
                        Button("Refresh") {
                            cgsManager.checkAccessibilityPermission()
                            cgsManager.refreshWindowList()
                        }
                        .font(.caption)
                        
                        Button("Create Test Window") {
                            cgsManager.createTestWindow()
                        }
                        .font(.caption)
                    }
                    
                    if !cgsManager.hasAccessibilityPermission {
                        HStack {
                            Text("Accessibility permission required")
                                .foregroundColor(.orange)
                                .font(.caption)
                            Button("Grant Permission") {
                                cgsManager.requestAccessibilityPermission()
                            }
                            .font(.caption)
                        }
                    }
                }
                
                Divider()
                
                // Window selection
                VStack(alignment: .leading) {
                    Text("Available Windows (\(cgsManager.allWindows.count))")
                        .font(.subheadline)
                        .fontWeight(.medium)
                    
                    Text("✅ Own windows should respond to all CGS APIs\n❌ Other windows may only respond to level/alpha changes")
                        .font(.caption2)
                        .foregroundColor(.secondary)
                        .padding(.vertical, 4)
                    
                    // Search field
                    HStack {
                        Image(systemName: "magnifyingglass")
                            .foregroundColor(.secondary)
                        TextField("Filter windows...", text: $searchText)
                            .textFieldStyle(RoundedBorderTextFieldStyle())
                    }
                    
                    ScrollView {
                        LazyVStack(spacing: 4) {
                            ForEach(filteredWindows) { window in
                                windowRow(window)
                            }
                        }
                    }
                }
                
                Divider()
                
                // Selected window info
                if let selected = cgsManager.selectedWindow {
                    selectedWindowInfo(selected)
                } else {
                    Text("Select a window to test CGS APIs")
                        .foregroundColor(.secondary)
                        .frame(maxWidth: .infinity, alignment: .center)
                }
            }
            .frame(width: 400)
            .padding()
            
            Divider()
            
            // Right panel: Test controls and log
            VStack(spacing: 16) {
                if let selected = cgsManager.selectedWindow {
                    testControlsSection(selected)
                } else {
                    Spacer()
                    Text("Select a window from the left panel to begin testing")
                        .foregroundColor(.secondary)
                        .font(.title2)
                    Spacer()
                }
                
                Divider()
                
                // Test log
                VStack(alignment: .leading) {
                    HStack {
                        Text("Test Log")
                            .font(.subheadline)
                            .fontWeight(.medium)
                        Spacer()
                        Button("Copy") {
                            NSPasteboard.general.clearContents()
                            NSPasteboard.general.setString(cgsManager.testLog, forType: .string)
                        }
                        .font(.caption)
                        .disabled(cgsManager.testLog.isEmpty)
                        
                        Button("Clear") {
                            cgsManager.clearLog()
                        }
                        .font(.caption)
                    }
                    
                    ScrollView {
                        ScrollViewReader { proxy in
                            Text(cgsManager.testLog.isEmpty ? "Test operations will appear here..." : cgsManager.testLog)
                                .font(.system(.caption, design: .monospaced))
                                .frame(maxWidth: .infinity, alignment: .leading)
                                .padding(8)
                                .background(Color.black.opacity(0.05))
                                .onChange(of: cgsManager.testLog) { _ in
                                    // Auto-scroll to bottom when new log entries are added
                                    withAnimation {
                                        proxy.scrollTo("bottom", anchor: .bottom)
                                    }
                                }
                                .id("logContent")
                            
                            // Invisible element to enable auto-scroll to bottom
                            HStack { }
                                .id("bottom")
                        }
                    }
                    .background(Color.gray.opacity(0.1))
                    .cornerRadius(4)
                }
                .frame(minHeight: 200)
            }
            .padding()
        }
    }
    
    // MARK: - View Components
    
    private func windowRow(_ window: CGSWindowManager.TestWindowInfo) -> some View {
        HStack {
            VStack(alignment: .leading, spacing: 2) {
                Text(window.displayName)
                    .font(.caption)
                    .lineLimit(1)
                
                HStack {
                    Text("ID: \(window.id)")
                    Text("Level: \(window.currentLevel)")
                    Text("Alpha: \(Int(window.currentAlpha * 100))%")
                }
                .font(.caption2)
                .foregroundColor(.secondary)
            }
            
            Spacer()
            
            HStack(spacing: 4) {
                if window.isOrderedOut {
                    Text("Hidden")
                        .font(.caption)
                        .padding(.horizontal, 6)
                        .padding(.vertical, 2)
                        .background(Color.orange.opacity(0.2))
                        .cornerRadius(4)
                }
                
                if window.isOwnWindow {
                    Text("✅ Own")
                        .font(.caption)
                        .padding(.horizontal, 6)
                        .padding(.vertical, 2)
                        .background(Color.green.opacity(0.2))
                        .cornerRadius(4)
                } else {
                    Text("❌ Other")
                        .font(.caption)
                        .padding(.horizontal, 6)
                        .padding(.vertical, 2)
                        .background(Color.red.opacity(0.2))
                        .cornerRadius(4)
                }
            }
        }
        .padding(.horizontal, 8)
        .padding(.vertical, 4)
        .background(
            cgsManager.selectedWindow?.id == window.id 
            ? Color.blue.opacity(0.1) 
            : Color.clear
        )
        .cornerRadius(4)
        .onTapGesture {
            cgsManager.selectedWindow = window
        }
    }
    
    private func selectedWindowInfo(_ window: CGSWindowManager.TestWindowInfo) -> some View {
        VStack(alignment: .leading, spacing: 8) {
            Text("Selected Window")
                .font(.subheadline)
                .fontWeight(.medium)
            
            VStack(alignment: .leading, spacing: 4) {
                Text(window.displayName)
                    .font(.caption)
                    .fontWeight(.medium)
                
                Text("ID: \(window.id)")
                    .font(.caption2)
                    .foregroundColor(.secondary)
                
                Text("Frame: \(Int(window.originalFrame.width)) × \(Int(window.originalFrame.height))")
                    .font(.caption2)
                    .foregroundColor(.secondary)
                
                Text("Current Level: \(window.currentLevel)")
                    .font(.caption2)
                    .foregroundColor(.secondary)
                
                Text("Current Alpha: \(Int(window.currentAlpha * 100))%")
                    .font(.caption2)
                    .foregroundColor(.secondary)
                
                if window.isOrderedOut {
                    Text("Status: Ordered Out")
                        .font(.caption2)
                        .foregroundColor(.orange)
                }
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
    }
    
    private func testControlsSection(_ window: CGSWindowManager.TestWindowInfo) -> some View {
        VStack(spacing: 16) {
            Text("CGS API Test Controls")
                .font(.title2)
                .fontWeight(.medium)
            
            // Quick actions
            VStack(spacing: 8) {
                Text("Quick Actions")
                    .font(.subheadline)
                    .fontWeight(.medium)
                
                LazyVGrid(columns: Array(repeating: GridItem(.flexible()), count: 2), spacing: 8) {
                    Button("Make Invisible") {
                        cgsManager.makeWindowInvisible(window.id)
                    }
                    .buttonStyle(.bordered)
                    
                    Button("Make Visible") {
                        cgsManager.makeWindowVisible(window.id)
                    }
                    .buttonStyle(.bordered)
                    
                    Button("Send to Back") {
                        cgsManager.sendWindowBehindDesktop(window.id)
                    }
                    .buttonStyle(.bordered)
                    
                    Button("Float Above") {
                        cgsManager.makeWindowFloat(window.id)
                    }
                    .buttonStyle(.bordered)
                    
                    Button("Restore Original") {
                        cgsManager.restoreWindow(window.id)
                    }
                    .buttonStyle(.borderedProminent)
                }
            }
            
            Divider()
            
            // Order controls
            VStack(spacing: 8) {
                Text("Window Ordering")
                    .font(.subheadline)
                    .fontWeight(.medium)
                
                HStack {
                    Button("Order Out") {
                        cgsManager.orderWindowOut(window.id)
                    }
                    .buttonStyle(.bordered)
                    
                    Button("Order In") {
                        cgsManager.orderWindowIn(window.id)
                    }
                    .buttonStyle(.bordered)
                    
                    Button("Order Below") {
                        cgsManager.orderWindowBelow(window.id)
                    }
                    .buttonStyle(.bordered)
                }
            }
            
            // Level controls
            VStack(spacing: 8) {
                Text("Window Level")
                    .font(.subheadline)
                    .fontWeight(.medium)
                
                LazyVGrid(columns: Array(repeating: GridItem(.flexible()), count: 3), spacing: 4) {
                    levelButton("Backstop", CGSWindowLevels.backstopMenu, window.id)
                    levelButton("Normal", CGSWindowLevels.normal, window.id)
                    levelButton("Floating", CGSWindowLevels.floating, window.id)
                    levelButton("Modal", CGSWindowLevels.modalPanel, window.id)
                    levelButton("Dock", CGSWindowLevels.dock, window.id)
                    levelButton("Overlay", CGSWindowLevels.overlay, window.id)
                }
                
                HStack {
                    Text("Custom:")
                    TextField("Level", text: $customLevel)
                        .textFieldStyle(.roundedBorder)
                        .frame(width: 80)
                    Button("Set") {
                        if let level = Int32(customLevel) {
                            cgsManager.setWindowLevel(window.id, level: level)
                        }
                    }
                    .buttonStyle(.bordered)
                }
            }
            
            // Alpha controls
            VStack(spacing: 8) {
                Text("Transparency (\(Int(alphaValue * 100))%)")
                    .font(.subheadline)
                    .fontWeight(.medium)
                
                HStack {
                    Slider(value: $alphaValue, in: 0...1, step: 0.01) {
                        Text("Alpha")
                    } onEditingChanged: { editing in
                        if !editing {
                            cgsManager.setWindowAlpha(window.id, alpha: alphaValue)
                        }
                    }
                }
                
                HStack {
                    ForEach([0, 25, 50, 75, 100], id: \.self) { percentage in
                        Button("\(percentage)%") {
                            alphaValue = Float(percentage) / 100.0
                            cgsManager.setWindowAlpha(window.id, alpha: alphaValue)
                        }
                        .buttonStyle(.bordered)
                        .font(.caption)
                    }
                }
            }
        }
    }
    
    private func levelButton(_ name: String, _ level: CGSWindowLevel, _ windowID: CGWindowID) -> some View {
        Button(name) {
            cgsManager.setWindowLevel(windowID, level: level)
        }
        .buttonStyle(.bordered)
        .font(.caption)
    }
}