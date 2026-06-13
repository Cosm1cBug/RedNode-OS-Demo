export * from './agent.js';
export const AGENTS = ['system','security','coding','research','automation','network'] as const;
export type AgentName = typeof AGENTS[number];
export interface ToolCall { tool: string; args: any }
