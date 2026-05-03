export interface InstanceConfig {
    app: string;
    version: string;
    baseUrl: string;
    documentation: string;
    repository: string;
    commit: string;
    sshPort?: number;
}
