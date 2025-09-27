import AppKit
import SwiftUI

struct ProjectSelectionView: View {
    @EnvironmentObject var permissionManager: PermissionManager
    @EnvironmentObject var agentManager: AgentManager
    @EnvironmentObject var settingsManager: SettingsManager
    let onProjectSelected: (String) -> Void
    @State private var showingNewProjectDialog = false
    @State private var showingDirectoryPicker = false
    @State private var pendingValidationError: ProjectValidationError? = nil

    private var hasValidAgent: Bool {
        agentManager.availableAgents.first(where: { $0.type == settingsManager.selectedAgent })?
            .isInstalled == true
            && agentManager.availableAgents.first(where: {
                $0.type == settingsManager.selectedAgent
            })?.isMCPConfigured == true
    }

    private var hasRequiredPermissions: Bool {
        permissionManager.hasAccessibilityPermission
            && permissionManager.hasScreenRecordingPermission
    }

    private func openProject(at path: String) {
        Logger.shared.log("ProjectSelectionView.openProject called with path: \(path)")
        onProjectSelected(path)
    }

    private var canCreateProjects: Bool {
        hasValidAgent && hasRequiredPermissions
    }

    var body: some View {
        VStack(spacing: 24) {
            // Header
            VStack(spacing: 8) {
                Image(systemName: "folder.badge.gearshape")
                    .font(.system(size: 48))
                    .foregroundColor(.blue)

                Text("Symposium")
                    .font(.largeTitle)
                    .fontWeight(.bold)

                Text("Select or create a project to get started")
                    .font(.headline)
                    .foregroundColor(.secondary)
            }

            // Action buttons
            VStack(spacing: 16) {
                Button(action: { showingNewProjectDialog = true }) {
                    HStack {
                        Image(systemName: "plus.circle.fill")
                        Text("Create New Project")
                    }
                    .frame(maxWidth: .infinity)
                    .padding()
                    .background(canCreateProjects ? Color.blue : Color.gray)
                    .foregroundColor(.white)
                    .cornerRadius(8)
                }
                .disabled(!canCreateProjects)

                Button(action: {
                    Logger.shared.log(
                        "Open Existing Project button clicked - showing directory picker directly")
                    showingDirectoryPicker = true
                }) {
                    HStack {
                        Image(systemName: "folder.circle")
                        Text("Open Existing Project")
                    }
                    .frame(maxWidth: .infinity)
                    .padding()
                    .background(
                        canCreateProjects ? Color.gray.opacity(0.2) : Color.gray.opacity(0.1)
                    )
                    .foregroundColor(canCreateProjects ? .primary : .secondary)
                    .cornerRadius(8)
                }
                .disabled(!canCreateProjects)
            }
            .frame(maxWidth: 300)

            // Status message when not ready
            if !canCreateProjects {
                VStack(spacing: 8) {
                    HStack {
                        Image(systemName: "exclamationmark.triangle.fill")
                            .foregroundColor(.orange)
                        Text("Setup Required")
                            .font(.headline)
                            .foregroundColor(.orange)
                    }

                    VStack(alignment: .leading, spacing: 4) {
                        if !hasRequiredPermissions {
                            Text("• Missing required permissions")
                                .font(.caption)
                                .foregroundColor(.secondary)
                        }
                        if !hasValidAgent {
                            Text("• No valid AI agent configured")
                                .font(.caption)
                                .foregroundColor(.secondary)
                        }
                        Text("Open Settings to complete setup")
                            .font(.caption)
                            .foregroundColor(.blue)
                    }
                }
                .padding()
                .background(Color.orange.opacity(0.1))
                .cornerRadius(8)
            }

            Spacer()
        }
        .padding(40)
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .fileImporter(
            isPresented: $showingDirectoryPicker,
            allowedContentTypes: [.folder],
            allowsMultipleSelection: false
        ) { result in
            switch result {
            case .success(let urls):
                Logger.shared.log("File picker succeeded with URLs: \(urls)")
                if let url = urls.first {
                    Logger.shared.log("Selected URL: \(url.path)")
                    
                    // Immediate validation with deferred user feedback
                    switch Project.validateProjectDirectory(url.path) {
                    case .success():
                        Logger.shared.log("Project validation successful, opening project")
                        openProject(at: url.path)
                    case .failure(let error):
                        Logger.shared.log("Project validation failed: \(error)")
                        // Store error to present after file picker closes
                        pendingValidationError = error
                    }
                } else {
                    Logger.shared.log("ERROR: No URL selected from file picker")
                }
            case .failure(let error):
                Logger.shared.log("ERROR: File picker failed: \(error)")
                print("Failed to select directory: \(error.localizedDescription)")
            }
        }
        .onChange(of: showingDirectoryPicker) { _, isShowing in
            // Present validation error alert after file picker closes
            if !isShowing, let error = pendingValidationError {
                Logger.shared.log("File picker closed, presenting validation error alert")
                DispatchQueue.main.asyncAfter(deadline: .now() + 0.1) {
                    ProjectValidationAlert.present(for: error)
                    pendingValidationError = nil
                }
            }
        }
        .onReceive(NotificationCenter.default.publisher(for: .showNewProjectDialog)) { _ in
            Logger.shared.log("Received showNewProjectDialog notification, opening new project dialog")
            showingNewProjectDialog = true
        }
        .sheet(isPresented: $showingNewProjectDialog) {
            NewProjectDialog(onProjectSelected: onProjectSelected)
        }
    }
}

struct NewProjectDialog: View {
    @EnvironmentObject var permissionManager: PermissionManager
    @EnvironmentObject var agentManager: AgentManager
    @EnvironmentObject var settingsManager: SettingsManager
    let onProjectSelected: (String) -> Void
    @Environment(\.dismiss) private var dismiss

    @State private var projectName = ""
    @State private var gitURL = ""
    @State private var selectedDirectory = ""
    @State private var selectedAgent: String? = nil
    @State private var defaultBranch = ""
    @State private var showingDirectoryPicker = false
    @State private var showingAdvancedSettings = false

    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            Text("Create New Project")
                .font(.headline)

            VStack(alignment: .leading, spacing: 8) {
                Text("Project Name:")
                TextField("Enter project name", text: $projectName)
                    .textFieldStyle(RoundedBorderTextFieldStyle())
            }

            VStack(alignment: .leading, spacing: 8) {
                Text("Git Repository URL:")
                TextField("https://github.com/user/repo.git", text: $gitURL)
                    .textFieldStyle(RoundedBorderTextFieldStyle())
            }

            VStack(alignment: .leading, spacing: 8) {
                Text("Location:")
                HStack {
                    Text(selectedDirectory.isEmpty ? "Select directory..." : selectedDirectory)
                        .foregroundColor(selectedDirectory.isEmpty ? .secondary : .primary)
                        .frame(maxWidth: .infinity, alignment: .leading)
                        .padding(8)
                        .background(Color.gray.opacity(0.1))
                        .cornerRadius(4)

                    Button("Browse") {
                        showingDirectoryPicker = true
                    }
                }
            }

            VStack(alignment: .leading, spacing: 8) {
                HStack {
                    Text("AI Agent:")
                    Spacer()
                    Button("Refresh") {
                        agentManager.scanForAgents(force: true)
                    }
                    .font(.caption)
                }
                VStack(alignment: .leading, spacing: 4) {
                    ForEach(agentManager.availableAgents, id: \.type) { agent in
                        AgentRadioButton(
                            agent: agent,
                            isSelected: selectedAgent == agent.type.id,
                            action: { selectedAgent = agent.type.id }
                        )
                    }

                    // None option
                    Button(action: { selectedAgent = "none" }) {
                        HStack {
                            Image(
                                systemName: selectedAgent == "none"
                                    ? "largecircle.fill.circle" : "circle"
                            )
                            .foregroundColor(selectedAgent == "none" ? .accentColor : .secondary)
                            Text("None")
                                .font(.subheadline)
                                .fontWeight(.medium)
                            Spacer()
                        }
                    }
                    .buttonStyle(PlainButtonStyle())
                }
            }

            // Advanced Settings
            VStack(alignment: .leading, spacing: 8) {
                Button(action: { showingAdvancedSettings.toggle() }) {
                    HStack {
                        Image(
                            systemName: showingAdvancedSettings ? "chevron.down" : "chevron.right")
                        Text("Advanced Settings")
                    }
                    .foregroundColor(.blue)
                }
                .buttonStyle(PlainButtonStyle())

                if showingAdvancedSettings {
                    VStack(alignment: .leading, spacing: 8) {
                        Text("Default Branch for New Taskspaces:")
                            .font(.caption)
                        TextField(
                            "Leave empty to use origin's default branch", text: $defaultBranch
                        )
                        .textFieldStyle(RoundedBorderTextFieldStyle())
                    }
                    .padding(.leading, 16)
                }
            }

            HStack {
                Button("Cancel") {
                    dismiss()
                }

                Spacer()

                Button("Create") {
                    createProject()
                }
                .disabled(
                    projectName.isEmpty || gitURL.isEmpty || selectedDirectory.isEmpty
                        || selectedAgent == nil)
            }
        }
        .padding()
        .frame(width: 400)
        .fileImporter(
            isPresented: $showingDirectoryPicker,
            allowedContentTypes: [.folder],
            allowsMultipleSelection: false
        ) { result in
            switch result {
            case .success(let urls):
                if let url = urls.first {
                    selectedDirectory = url.path
                }
            case .failure(let error):
                // Could show alert here
                print("Failed to select directory: \(error.localizedDescription)")
            }
        }
    }

    private func createProject() {
        do {
            let agent = (selectedAgent == "none") ? nil : selectedAgent
            let branch = defaultBranch.isEmpty ? nil : defaultBranch
            try createProject(
                name: projectName, gitURL: gitURL, at: selectedDirectory,
                agent: agent, defaultBranch: branch)
            dismiss() // Dismiss the sheet first
            onProjectSelected("\(selectedDirectory)/\(projectName).symposium")
        } catch {
            // Handle error - could show alert
        }
    }

    /// Create a new Symposium project
    private func createProject(
        name: String, gitURL: String, at directoryPath: String, agent: String? = nil,
        defaultBranch: String? = nil
    ) throws {
        // Create project directory with .symposium extension
        let projectDirPath = "\(directoryPath)/\(name).symposium"

        // Check if directory already exists
        if FileManager.default.fileExists(atPath: projectDirPath) {
            throw ProjectError.directoryAlreadyExists
        }

        // Create directory
        try FileManager.default.createDirectory(
            atPath: projectDirPath,
            withIntermediateDirectories: true,
            attributes: nil
        )

        // Create project instance
        let project = Project(
            name: name, gitURL: gitURL, directoryPath: projectDirPath, agent: agent,
            defaultBranch: defaultBranch)

        // Save project.json
        try project.save()
    }
}
