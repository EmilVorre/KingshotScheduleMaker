import { createContext, useContext, useEffect, useState, useCallback, type ReactNode } from 'react'
import { api } from '../api/client'

interface AuthState {
  accountName: string | null
  serverNumber: number | null
  playerId: string | null
  inGameName: string | null
  isAdmin: boolean
  allianceAccess: boolean
  friendCode: string | null
  isValid: boolean | null
}

interface AuthContextValue extends AuthState {
  refresh: () => Promise<void>
}

const AuthContext = createContext<AuthContextValue | null>(null)

export function AuthProvider({ children }: { children: ReactNode }) {
  const [state, setState] = useState<AuthState>({
    accountName: null,
    serverNumber: null,
    playerId: null,
    inGameName: null,
    isAdmin: false,
    allianceAccess: false,
    friendCode: null,
    isValid: null,
  })

  const refresh = useCallback(async () => {
    const { ok, data } = await api.getSession()
    if (ok && data?.account_name) {
      setState({
        accountName: data.account_name,
        serverNumber: data.server_number ?? null,
        playerId: data.player_id ?? null,
        inGameName: data.in_game_name ?? null,
        isAdmin: data.is_admin ?? false,
        allianceAccess: data.alliance_access ?? false,
        friendCode: data.friend_code ?? null,
        isValid: true,
      })
    } else {
      setState({
        accountName: null,
        serverNumber: null,
        playerId: null,
        inGameName: null,
        isAdmin: false,
        allianceAccess: false,
        friendCode: null,
        isValid: false,
      })
    }
  }, [])

  useEffect(() => {
    refresh()
  }, [refresh])

  return (
    <AuthContext.Provider value={{ ...state, refresh }}>
      {children}
    </AuthContext.Provider>
  )
}

export function useAuth() {
  const ctx = useContext(AuthContext)
  if (!ctx) throw new Error('useAuth must be used within AuthProvider')
  return ctx
}
