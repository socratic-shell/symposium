import AppKit
import SwiftUI

struct SettingsView: View {
    @EnvironmentObject var permissionManager: PermissionManager
    @EnvironmentObject var agentManager: AgentManager
    @EnvironmentObject var settingsManager: SettingsManager
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
                .disabled(!allRequiredPermissionsGranted || !hasValidAgentSelected)
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
                HStack {
                    Text("AI Agent")
                        .font(.headline)

                    if agentManager.scanningInProgress {
                        ProgressView()
                            .scaleEffect(0.7)
                    }

                    Spacer()

                    Button("Refresh") {
                        agentManager.scanForAgents(force: true)
                    }
                    .buttonStyle(.bordered)
                    .controlSize(.small)
                }

                Text("Choose which AI agent to use for taskspaces:")
                    .font(.subheadline)
                    .foregroundColor(.secondary)

                VStack(alignment: .leading, spacing: 8) {
                    ForEach(agentManager.availableAgents) { agent in
                        AgentRadioButton(
                            agent: agent,
                            isSelected: settingsManager.selectedAgent == agent.type,
                            action: {
                                if agent.isInstalled && agent.isMCPConfigured {
                                    settingsManager.selectedAgent = agent.type
                                }
                            }
                        )
                    }

                    if agentManager.availableAgents.isEmpty && !agentManager.scanningInProgress {
                        HStack {
                            Image(systemName: "exclamationmark.triangle.fill")
                                .foregroundColor(.orange)
                            Text("No compatible AI agents found")
                                .font(.subheadline)
                                .foregroundColor(.secondary)
                        }
                        .padding(8)
                    }
                }
            }

            Spacer()

            // Status message
            if !allRequiredPermissionsGranted || !hasValidAgentSelected {
                HStack {
                    Image(systemName: "exclamationmark.triangle.fill")
                        .foregroundColor(.orange)

                    VStack(alignment: .leading, spacing: 2) {
                        if !allRequiredPermissionsGranted {
                            Text("Required permissions must be granted")
                                .font(.caption)
                                .foregroundColor(.secondary)
                        }
                        if !hasValidAgentSelected {
                            Text("A properly configured AI agent must be selected")
                                .font(.caption)
                                .foregroundColor(.secondary)
                        }
                    }
                }
            }
        }
        .padding(24)
        .frame(
            minWidth: 700, idealWidth: 800, maxWidth: 1000,
            minHeight: 600, idealHeight: 700, maxHeight: 900
        )
        .onAppear {
            permissionManager.checkAllPermissions()
        }
    }

    private var allRequiredPermissionsGranted: Bool {
        permissionManager.hasAccessibilityPermission
            && permissionManager.hasScreenRecordingPermission
    }

    private var hasValidAgentSelected: Bool {
        guard
            let selectedAgentInfo = agentManager.availableAgents.first(where: {
                $0.type == settingsManager.selectedAgent
            })
        else {
            return false
        }
        return selectedAgentInfo.isInstalled && selectedAgentInfo.isMCPConfigured
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

struct AgentRadioButton: View {
    let agent: AgentInfo
    let isSelected: Bool
    let action: () -> Void

    var body: some View {
        Button(action: action) {
            HStack {
                Image(systemName: isSelected ? "largecircle.fill.circle" : "circle")
                    .foregroundColor(isSelected ? .accentColor : .secondary)

                VStack(alignment: .leading, spacing: 4) {
                    HStack {
                        Text(agent.name)
                            .font(.subheadline)
                            .fontWeight(.medium)

                        Spacer()

                        Image(systemName: agent.statusIcon)
                            .foregroundColor(Color(agent.statusColor))

                        Text(agent.statusText)
                            .font(.caption)
                            .foregroundColor(Color(agent.statusColor))
                    }

                    Text(agent.description)
                        .font(.caption)
                        .foregroundColor(.secondary)

                    if let mcpPath = agent.mcpServerPath {
                        Text("MCP Server: \(mcpPath)")
                            .font(.caption2)
                            .foregroundColor(.secondary)
                            .lineLimit(1)
                            .truncationMode(.middle)
                    }
                }

                Spacer()
            }
        }
        .buttonStyle(.plain)
        .disabled(!agent.isInstalled || !agent.isMCPConfigured)
        .padding(8)
        .background(isSelected ? Color.accentColor.opacity(0.1) : Color.clear)
        .cornerRadius(6)
    }
}
