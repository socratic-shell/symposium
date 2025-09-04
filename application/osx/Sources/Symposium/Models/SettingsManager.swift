import Foundation
import SwiftUI

class SettingsManager: ObservableObject {
    @AppStorage("selectedAgent") var selectedAgentRaw: String = AgentType.qcli.rawValue
    @AppStorage("lastProjectPath") var lastProjectPath: String = ""
    
    var selectedAgent: AgentType {
        get { AgentType(rawValue: selectedAgentRaw) ?? .qcli }
        set { selectedAgentRaw = newValue.rawValue }
    }
    
    // Add other settings here as needed
    // @AppStorage("someOtherSetting") var someOtherSetting: Bool = false
}
