import { createBrowserRouter, type RouteObject } from 'react-router'

import Layout from './_layout'
import { navItems } from './_navigation'

export const router = createBrowserRouter([
  {
    path: '/',
    Component: Layout,
    children: navItems.map((item) => {
      const route: RouteObject = { path: item.path }
      if (item.Component) {
        route.Component = item.Component
      }
      if (item.lazy) {
        route.lazy = item.lazy
      }
      return route
    }),
  },
])
