import Foundation
import SwiftUI

class SettingsManager: ObservableObject {
    @AppStorage("selectedAgent") var selectedAgent: String = "qcli"
    @AppStorage("lastProjectPath") var lastProjectPath: String = ""
    
    // Add other settings here as needed
    // @AppStorage("someOtherSetting") var someOtherSetting: Bool = false
}
