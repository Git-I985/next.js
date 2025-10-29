import type * as Playwright from 'playwright'
import { nextTestSetup } from 'e2e-utils'
import { createRouterAct } from 'router-act'
import { setTimeout } from 'node:timers/promises'

describe('router-act', () => {
  const { next } = nextTestSetup({
    files: __dirname,
  })

  describe('prints debug info when test times out on act()', () => {
    let didTimeout = false
    let consoleErrorMock: jest.SpyInstance
    beforeAll(() => {
      consoleErrorMock = jest
        .spyOn(console, 'error')
        .mockImplementation(() => {})
    })
    afterAll(() => {
      consoleErrorMock?.mockClear()
    })

    test('example', async () => {
      let page: Playwright.Page
      await next.browser('/', {
        beforePageLoad(p: Playwright.Page) {
          page = p
        },
      })
      const act = createRouterAct(page)

      let abortAct
      const actAborted = new Promise<void>((resolve) => {
        abortAct = resolve
      })
      // Navigation is to a slow page
      await Promise.race([
        act(async () => {
          await page.click('text=Go to slow')
          // wait for requests to init, and then abort
          setTimeout(1000).then(abortAct)
        }),
        actAborted.then(() => {
          didTimeout = true
        }),
      ])
    })

    test('assertion', async () => {
      expect({ didTimeout }).toEqual({ didTimeout: true })
      // The timing is hard to simulate. We just want to make sure the abort handler was called.
      expect(consoleErrorMock).toHaveBeenCalledTimes(1)
    })
  })
})
