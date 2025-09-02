import SwiftUI

struct SettingsView: View {
    @StateObject private var permissionManager = PermissionManager()
    @AppStorage("selectedAgent") private var selectedAgent: String = "qcli"
    @Environment(\.dismiss) private var dismiss
    
    var body: some View {
        VStack(alignment: .leading, spacing: 20) {
            // Header
            HStack {
                Text("Symposium Settings")
                    .font(.title2)
                    .fontWeight(.bold)
                
                Spacer()
                
                Button("Done") {
                    dismiss()
                }
                .disabled(!allRequiredPermissionsGranted)
            }
            
            Divider()
            
            // Permissions Section
            VStack(alignment: .leading, spacing: 16) {
                Text("Permissions")
                    .font(.headline)
                
                // Accessibility Permission
                PermissionRow(
                    title: "Accessibility",
                    description: "Required to manage and tile windows",
                    isGranted: permissionManager.hasAccessibilityPermission,
                    isRequired: true,
                    onRequest: {
                        permissionManager.requestAccessibilityPermission()
                    },
                    onOpenSettings: {
                        permissionManager.openSystemPreferences(for: .accessibility)
                    }
                )
                
                // Screen Recording Permission
                PermissionRow(
                    title: "Screen Recording",
                    description: "Required for taskspace screenshots",
                    isGranted: permissionManager.hasScreenRecordingPermission,
                    isRequired: true,
                    onRequest: {
                        permissionManager.requestScreenRecordingPermission()
                    },
                    onOpenSettings: {
                        permissionManager.openSystemPreferences(for: .screenRecording)
                    }
                )
            }
            
            Divider()
            
            // Agent Selection Section
            VStack(alignment: .leading, spacing: 16) {
                Text("AI Agent")
                    .font(.headline)
                
                Text("Choose which AI agent to use for taskspaces:")
                    .font(.subheadline)
                    .foregroundColor(.secondary)
                
                VStack(alignment: .leading, spacing: 8) {
                    RadioButton(
                        title: "Q CLI",
                        description: "Amazon Q Developer CLI",
                        isSelected: selectedAgent == "qcli",
                        action: { selectedAgent = "qcli" }
                    )
                    
                    RadioButton(
                        title: "Claude Code",
                        description: "Anthropic Claude for coding",
                        isSelected: selectedAgent == "claude",
                        action: { selectedAgent = "claude" }
                    )
                }
            }
            
            Spacer()
            
            // Status message
            if !allRequiredPermissionsGranted {
                HStack {
                    Image(systemName: "exclamationmark.triangle.fill")
                        .foregroundColor(.orange)
                    Text("Required permissions must be granted before using Symposium")
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
            }
        }
        .padding(24)
        .frame(width: 500, height: 400)
        .onAppear {
            permissionManager.checkAllPermissions()
        }
    }
    
    private var allRequiredPermissionsGranted: Bool {
        permissionManager.hasAccessibilityPermission && permissionManager.hasScreenRecordingPermission
    }
}

struct PermissionRow: View {
    let title: String
    let description: String
    let isGranted: Bool
    let isRequired: Bool
    let onRequest: () -> Void
    let onOpenSettings: () -> Void
    
    var body: some View {
        HStack {
            VStack(alignment: .leading, spacing: 4) {
                HStack {
                    Text(title)
                        .font(.subheadline)
                        .fontWeight(.medium)
                    
                    if isRequired {
                        Text("Required")
                            .font(.caption)
                            .padding(.horizontal, 6)
                            .padding(.vertical, 2)
                            .background(Color.red.opacity(0.1))
                            .foregroundColor(.red)
                            .cornerRadius(4)
                    }
                }
                
                Text(description)
                    .font(.caption)
                    .foregroundColor(.secondary)
            }
            
            Spacer()
            
            HStack(spacing: 8) {
                // Status indicator
                Image(systemName: isGranted ? "checkmark.circle.fill" : "xmark.circle.fill")
                    .foregroundColor(isGranted ? .green : .red)
                
                Text(isGranted ? "Granted" : "Required")
                    .font(.caption)
                    .fontWeight(.medium)
                    .foregroundColor(isGranted ? .green : .red)
                
                if !isGranted {
                    Button("Grant") {
                        onRequest()
                    }
                    .buttonStyle(.bordered)
                    .controlSize(.small)
                    
                    Button("Open Settings") {
                        onOpenSettings()
                    }
                    .buttonStyle(.bordered)
                    .controlSize(.small)
                }
            }
        }
        .padding(12)
        .background(Color.gray.opacity(0.05))
        .cornerRadius(8)
    }
}

struct RadioButton: View {
    let title: String
    let description: String
    let isSelected: Bool
    let action: () -> Void
    
    var body: some View {
        Button(action: action) {
            HStack {
                Image(systemName: isSelected ? "largecircle.fill.circle" : "circle")
                    .foregroundColor(isSelected ? .accentColor : .secondary)
                
                VStack(alignment: .leading, spacing: 2) {
                    Text(title)
                        .font(.subheadline)
                        .fontWeight(.medium)
                        .foregroundColor(.primary)
                    
                    Text(description)
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
                
                Spacer()
            }
        }
        .buttonStyle(.plain)
        .padding(8)
        .background(isSelected ? Color.accentColor.opacity(0.1) : Color.clear)
        .cornerRadius(6)
    }
}
