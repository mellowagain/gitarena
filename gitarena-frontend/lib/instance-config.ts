export interface InstanceConfig {
    app: string;
    version: string;
    baseUrl: string;
    backendUrl: string;
    userEmailDomain: string;
    documentation: string;
    repository: string;
    commit: string;
    sshPort?: number;
    objectStorageAvailable: boolean;
    smtpEnabled: boolean;
}
