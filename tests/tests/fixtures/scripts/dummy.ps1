if ($Env:TEST_VAR -ne 'test message') { 
	throw 'Error, env var TEST does not  exist'
}